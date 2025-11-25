use crate::cli::OutputFormat;
use crate::common::CommonOpts;

/// Run the `stats` command.
pub fn run(format: &OutputFormat, common: &CommonOpts) {
    println!("[stats] format: {format:#?}");
    println!("[stats] common: {common:#?}");
}
