use anyhow::{Context, Result};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::common::{CommonOpts, LogLine, build_filters, detect_level, extract_log_timestamp};

/// Run the `tail` command.
// ponytail: single-file polling, no rotation/truncate. Add when needed.
pub fn run(path: &PathBuf, interval: f32, from_start: bool, common: &CommonOpts) -> Result<()> {
    let filters = build_filters(common)?;

    let source: Arc<str> = Arc::from(
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown"),
    );

    let file = File::open(path)
        .with_context(|| format!("Failed to open '{}'", path.display()))?;
    let mut pos: u64 = if from_start {
        0
    } else {
        file.metadata()?.len()
    };

    let mut reader = BufReader::new(file);
    let sleep = Duration::from_secs_f32(interval);
    let stdout = io::stdout();
    let mut line = String::new();

    loop {
        reader.seek(SeekFrom::Start(pos))?;
        loop {
            line.clear();
            let n = reader.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            // Wait until the line is complete (ends with newline).
            if !line.ends_with('\n') {
                break;
            }
            pos += n as u64;
            let raw = line.trim_end_matches(['\n', '\r']).to_owned();
            let ts = extract_log_timestamp(&raw, &common.date_format);
            let lvl = detect_level(&raw);
            let log = LogLine::new(source.clone(), raw, ts, lvl);
            if filters.matches(&log) {
                let mut out = stdout.lock();
                writeln!(out, "{}", log.raw)?;
                out.flush()?;
            }
        }
        thread::sleep(sleep);
    }
}
