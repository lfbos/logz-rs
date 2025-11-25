use anyhow::Result;
use std::io;
use std::path::PathBuf;
use crate::common::{CommonOpts, build_filters};

/// Run the `analyze` command.
pub fn run(path: Option<PathBuf>, out: Option<PathBuf>, common: &CommonOpts) -> Result<()> {
    let _filters = build_filters(common)?;

    let file_path: PathBuf;

    match path {
        Some(path) => {
            file_path = path;
        }
        None => {
            println!("Reading from stdin (press Ctrl+D to end, or Ctrl+C to cancel)...");
            let mut user_input = String::new();
            io::stdin()
                .read_line(&mut user_input)
                .expect("Failed to read line");
            file_path = PathBuf::from(user_input.trim())
        }
    }

    println!("{:?}", file_path);
    
    Ok(())
}
