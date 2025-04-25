//! Singularity

use std::process::Output;

use indexmap::IndexMap;

const COMMAND_BASE: &str = "singularity";

/// A configuration for the host.
#[derive(Clone, Debug)]
pub struct HostConfig {
    /// CPU shares for the container.
    pub cpu_shares: Option<u64>,
    /// Number of CPUs available to the container
    pub cpus: Option<u64>,
    /// Memory limit for the container.
    pub memory: Option<u64>,
    /// Memory reservation for the container.
    pub memory_reservation: Option<u64>,
    /// Bind mounts for the container.
    pub binds: Option<Vec<(String, String)>>,
    /// Contain file systems, PID, IPC, and environment.
    pub contain_all: bool,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            cpu_shares: None,
            cpus: Some(1),
            memory: None,
            memory_reservation: Some(2 * 1024 * 1024 * 1024), // 2 GiB
            binds: None,
            contain_all: true,
        }
    }
}

/// A Singularity client.
#[derive(Clone, Debug, Default)]
pub struct Singularity {
    /// The image (e.g., `ubuntu:latest`).
    image: String,

    /// The program to run.
    program: String,

    /// The arguments to the command.
    args: Vec<String>,

    /// Whether or not the standard output is attached.
    attach_stdout: bool,

    /// Whether or not the standard error is attached.
    attach_stderr: bool,

    /// Environment variables.
    env: IndexMap<String, String>,

    /// The working directory.
    work_dir: Option<String>,

    /// Host configuration.
    host_config: Option<HostConfig>,
}

impl Singularity {
    /// Creates a new [`Singularity`] client.
    pub fn new() -> Self {
        Self {
            image: Default::default(),
            program: Default::default(),
            args: Default::default(),
            attach_stdout: false,
            attach_stderr: false,
            env: Default::default(),
            work_dir: Default::default(),
            host_config: Default::default(),
        }
    }

    /// Adds an image name.
    pub fn image(mut self, image: impl Into<String>) -> Self {
        self.image = image.into();
        self
    }

    /// Adds a program to run.
    pub fn program(mut self, program: impl Into<String>) -> Self {
        self.program = program.into();
        self
    }

    /// Adds an argument.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Sets multiple arguments.
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Sets stdout to be attached.
    pub fn attach_stdout(mut self) -> Self {
        self.attach_stdout = true;
        self
    }

    /// Sets stderr to be attached.
    pub fn attach_stderr(mut self) -> Self {
        self.attach_stderr = true;
        self
    }

    /// Sets an environment variable.
    pub fn env(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(name.into(), value.into());
        self
    }

    /// Sets multiple environment variables.
    pub fn envs(
        mut self,
        variables: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.env
            .extend(variables.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }

    /// Sets the working directory.
    pub fn work_dir(mut self, work_dir: impl Into<String>) -> Self {
        self.work_dir = Some(work_dir.into());
        self
    }

    /// Sets the host configuration.
    pub fn host_config(mut self, host_config: HostConfig) -> Self {
        self.host_config = Some(host_config);
        self
    }

    /// Pulls a Singularity image from a given URL.
    pub fn pull_image(
        &self,
        image: &str,
        output_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check if the image is already pulled
        if std::path::Path::new(output_path).exists() {
            println!("Image already pulled: {}", image);
            return Ok(());
        }
        // Pull the image using Singularity
        let mut cmd = std::process::Command::new(COMMAND_BASE);
        cmd.arg("pull")
            .arg(output_path)
            .arg(image);

        // Execute the command and capture the output
        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    Ok(())
                } else {
                    Err(format!(
                        "Failed to pull image: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ).into())
                }
            }
            Err(e) => Err(format!("Failed to execute Singularity: {}", e).into()),
        }
    }

    /// Executes a command inside a Singularity container.
    pub fn exec(&self, binds: Vec<(String, String)>, args: Vec<String>) -> Result<Output, Box<dyn std::error::Error>> {
        let mut cmd = std::process::Command::new(COMMAND_BASE);
        cmd.arg("exec");

        // Add bind mounts if provided
        for (host_path, container_path) in 
            self.host_config
                .as_ref()
                .and_then(|h| h.binds.clone())
                .unwrap_or_else(|| binds)
        {
            cmd.arg(format!("--bind {}:{}", host_path, container_path));
        }

        // Add cpu_shares if provided
        if let Some(cpu_shares) = self.host_config.as_ref().and_then(|h| h.cpu_shares) {
            cmd.arg(format!("--cpu-shares={}", cpu_shares));
        }

        // Add cpus if provided
        if let Some(cpus) = self.host_config.as_ref().and_then(|h| h.cpus) {
            cmd.arg(format!("--cpus={}", cpus));
        }

        // Add memory if provided
        if let Some(memory) = self.host_config.as_ref().and_then(|h| h.memory) {
            cmd.arg(format!("--memory={}", memory));
        }

        // Add memory reservation if provided
        if let Some(memory_reservation) = self.host_config.as_ref().and_then(|h| h.memory_reservation) {
            cmd.arg(format!("--memory-reservation={}", memory_reservation));
        }

        // Add environment variables if provided
        for (key, value) in &self.env {
            cmd.arg(format!("--env {}={}", key, value));
        }

        // Add the working directory if provided
        if let Some(work_dir) = &self.work_dir {
            cmd.arg(format!("--workdir {}", work_dir));
        }

        // Add contain options if provided
        if let Some(host_config) = &self.host_config {
            if host_config.contain_all {
                cmd.arg("--containall");
            }
        }

        // Add additional arguments if provided
        for arg in args {
            cmd.arg(arg);
        }

        // Add the image name
        cmd.arg(self.image.as_str());

        // Add the command to run
        cmd.arg(self.program.as_str());

        // Add the command arguments
        for arg in &self.args {
            cmd.arg(arg);
        }

        println!("executing command: {:?}", cmd);

        // Execute the command and capture the output
        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    println!("Output: {}", String::from_utf8_lossy(&output.stdout));
                    Ok(output)
                } else {
                    Err(format!(
                        "Failed to execute command: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ).into())
                }
            }
            Err(e) => Err(format!("Failed to execute Singularity: {}", e).into()),
        }
    }

    /// Gets the version of Singularity.
    pub fn version(&self) -> Result<String, Box<dyn std::error::Error>> {
        let output = std::process::Command::new(COMMAND_BASE)
            .arg("--version")
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(format!(
                "Failed to get Singularity version: {}",
                String::from_utf8_lossy(&output.stderr)
            ).into())
        }
    }
}