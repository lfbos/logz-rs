use crate::common::{CommonOpts, build_filters, get_log_lines};
use anyhow::Result;
use std::io;
use std::path::PathBuf;

/// Run the `analyze` command.
pub fn run(path: Option<PathBuf>, out: Option<PathBuf>, common: &CommonOpts) -> Result<()> {
    let filters = build_filters(common)?;

    let file_path: PathBuf;
    let mut input_from_stdin: bool = false;

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
            input_from_stdin = true;
            file_path = PathBuf::from(user_input.trim())
        }
    }

    if !file_path.exists() {
        return Err(anyhow::anyhow!(
            "File does not exist: {}",
            file_path.display()
        ));
    }

    let log_lines = get_log_lines(file_path, input_from_stdin, &common.date_format);
    println!("[analyze] log_lines: {:?}", log_lines);
    Ok(())
}
