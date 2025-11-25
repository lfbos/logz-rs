use clap::{error::ErrorKind, CommandFactory, Parser};
use logz_rs::cli::{Cli, Commands};
use logz_rs::commands::{analyze, stats, tail};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Analyze { path, out, common } => {
            analyze::run(path, out, &common)
        }
        Commands::Stats { format, common } => {
            stats::run(&format, &common)
        }
        Commands::Tail {
            path,
            interval,
            from_start,
            common,
        } => {
            tail::run(&path, interval, from_start, &common)
        }
    };

    if let Err(err) = result {
        let mut cmd = Cli::command();
        cmd.error(
            ErrorKind::ValueValidation,
            format!("{:#}", err)
        ).exit();
    }
}
