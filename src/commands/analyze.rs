use crate::common::{CommonOpts, build_filters, get_log_lines, resolve_input_path};
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

/// Run the `analyze` command.
pub fn run(path: Option<PathBuf>, out: Option<PathBuf>, common: &CommonOpts) -> Result<()> {
    let filters = build_filters(common)?;
    let (file_path, input_from_stdin) = resolve_input_path(path)?;

    let log_stream = get_log_lines(file_path, input_from_stdin, &common.date_format, filters)?;

    let mut writer: Box<dyn Write> = match &out {
        Some(p) => {
            let f = File::create(p)
                .with_context(|| format!("Failed to create output file '{}'", p.display()))?;
            Box::new(BufWriter::new(f))
        }
        None => Box::new(BufWriter::new(io::stdout().lock())),
    };

    let mut processed = 0usize;
    for log_line in log_stream {
        let line = log_line?;
        writeln!(writer, "{}", line.raw)?;
        processed += 1;
    }
    writer.flush()?;

    eprintln!("[analyze] processed {} filtered log lines", processed);
    Ok(())
}
