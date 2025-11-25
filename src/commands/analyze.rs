use std::path::PathBuf;

use crate::common::CommonOpts;

/// Run the `analyze` command.
pub fn run(out: &PathBuf, common: &CommonOpts) {
    // Here you will implement the real analyze logic.
    println!("[analyze] out: {out:#?}");
    println!("[analyze] common: {common:#?}");
}
