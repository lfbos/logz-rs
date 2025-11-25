use clap::Parser;
use logz_rs::cli::{Cli, Commands};
use logz_rs::commands::{analyze, stats, tail};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { out, common } => {
            analyze::run(&out, &common);
        }
        Commands::Stats { format, common } => {
            stats::run(&format, &common);
        }
        Commands::Tail {
            path,
            interval,
            from_start,
            common,
        } => {
            tail::run(&path, interval, from_start, &common);
        }
    }
}
