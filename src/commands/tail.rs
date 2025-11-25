use std::path::PathBuf;

use crate::common::CommonOpts;

/// Run the `tail` command.
pub fn run(path: &PathBuf, interval: f32, from_start: bool, common: &CommonOpts) {
    println!("[tail] path: {path:#?}");
    println!("[tail] interval: {interval}");
    println!("[tail] from_start: {from_start}");
    println!("[tail] common: {common:#?}");
}
