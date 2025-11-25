use clap::{Parser, Subcommand, ValueEnum};
use std::path::{PathBuf};

use crate::common::CommonOpts;

#[derive(Parser)]
#[command(name = "logz")]
#[command(version = "1.0")]
#[command(about = "Logs Analyzer in Rust")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum OutputFormat {
    Json,
    Markdown,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Analyze {
        #[arg(long, required = false)]
        path: Option<PathBuf>,

        #[arg(
            long,
            help = "Output file to write filtered lines. If omitted, prints to stdout."
        )]
        out: Option<PathBuf>,

        #[command(flatten)]
        common: CommonOpts,
    },
    Stats {
        #[arg(
            short,
            long = "format",
            default_value = "json",
            help = "Output format for stats."
        )]
        format: OutputFormat,

        #[command(flatten)]
        common: CommonOpts,
    },
    Tail {
        #[arg(long, required = true)]
        path: PathBuf,

        #[arg(long, default_value_t = 0.5, help = "Polling interval in seconds.")]
        interval: f32,

        #[arg(long = "from-start")]
        from_start: bool,

        #[command(flatten)]
        common: CommonOpts,
    },
}
