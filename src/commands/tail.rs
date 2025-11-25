use anyhow::Result;
use std::path::PathBuf;

use crate::common::{CommonOpts, build_filters};

/// Run the `tail` command.
pub fn run(path: &PathBuf, interval: f32, from_start: bool, common: &CommonOpts) -> Result<()> {
    let _filters = build_filters(common)?;
    
    println!("[tail] path: {path:#?}");
    println!("[tail] interval: {interval}");
    println!("[tail] from_start: {from_start}");
    println!("[tail] common: {common:#?}");
    
    Ok(())
}
