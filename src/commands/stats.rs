use crate::cli::OutputFormat;
use crate::common::{CommonOpts, build_filters};
use anyhow::Result;

/// Run the `stats` command.
pub fn run(format: &OutputFormat, common: &CommonOpts) -> Result<()> {
    let _filters = build_filters(common)?;

    println!("[stats] format: {format:#?}");
    println!("[stats] common: {common:#?}");

    Ok(())
}
