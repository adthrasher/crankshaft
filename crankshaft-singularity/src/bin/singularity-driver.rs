//! A command line tool to test the [`crankshaft_singularity`] crate.
//!
//! This binary will typically only be useful to developers of this crate.
#![allow(missing_docs)]
#![allow(clippy::missing_docs_in_private_items)]

use clap::Parser;
use clap::Subcommand;
use clap_verbosity_flag::Verbosity;
use crankshaft_singularity::Singularity;
use eyre::Result;
use eyre::eyre;
use shlex;
use tracing_log::AsTrace;
use tracing_subscriber::EnvFilter;

#[derive(clap::Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,

    #[command(flatten)]
    verbose: Verbosity,
}

#[derive(Subcommand)]
enum Command {
    /// Pulls a Singularity image from a given URL.
    PullImage {
        /// The URL of the image.
        image: String,

        /// The output path for the image.
        output_path: String,
    },
    /// Runs a container with a particular command and prints the result.
    RunContainer {
        /// The name of the image.
        image: String,

        /// The command to run.
        command: String,
    },
}

async fn run(args: Args) -> Result<()> {
    let singularity = Singularity::default();

    match args.command {
        Command::PullImage { image, output_path } => {
            let _ = singularity.pull_image(&image, &output_path); //.await?;
        }
        Command::RunContainer {
            image,
            command,
        } => {
            // Split the command into parts
            let mut command =
                shlex::split(&command).ok_or_else(|| eyre!("invalid command `{command}`"))?;
            let args = command.split_off(1);

            let singularity = singularity.image(image.clone())
                .program(command.remove(0))
                .args(args);

            match singularity.exec(vec![], vec![]) {
                Ok(output) => {
                    println!("Success: {}", String::from_utf8_lossy(&output.stdout));
                }
                Err(e) => {
                    return Err(eyre!("Failed to execute command: {}", e));
                }
            } //.await?;
        }
    };

    Ok(())
}

pub fn main() -> Result<()> {
    let args = Args::parse();

    match std::env::var("RUST_LOG") {
        Ok(_) => tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init(),
        Err(_) => tracing_subscriber::fmt()
            .with_max_level(args.verbose.log_level_filter().as_trace())
            .init(),
    };

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run(args))
}
