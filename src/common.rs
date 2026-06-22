use std::{
    cmp::min,
    fs::{self, File},
    io::{self, BufRead, BufReader, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use clap::Args;
use flate2::read::GzDecoder;

pub const DEFAULT_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const LEVELS: [&str; 6] = ["DEBUG", "INFO", "WARN", "WARNING", "ERROR", "CRITICAL"];

#[derive(Clone)]
pub struct LogFilters {
    pub from_ts: Option<DateTime<Utc>>,
    pub to_ts: Option<DateTime<Utc>>,
    pub regex: Option<regex::Regex>,
    pub levels: Vec<String>,
    pub substring_match: Option<String>,
}

impl LogFilters {
    pub fn matches(&self, log_line: &LogLine) -> bool {
        if let Some(ref from) = self.from_ts {
            match log_line.timestamp.as_ref() {
                Some(ts) if ts < from => return false,
                None => return false,
                _ => {}
            }
        }

        if let Some(ref to) = self.to_ts {
            match log_line.timestamp.as_ref() {
                Some(ts) if ts > to => return false,
                None => return false,
                _ => {}
            }
        }

        if !self.levels.is_empty() {
            match log_line.level.as_ref() {
                Some(level) => {
                    if !self.levels.iter().any(|expected| expected == level) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        if let Some(ref needle) = self.substring_match
            && !log_line.raw.contains(needle)
        {
            return false;
        }

        if let Some(ref regex) = self.regex
            && !regex.is_match(&log_line.raw)
        {
            return false;
        }

        true
    }
}

#[derive(Debug)]
pub struct LogLine {
    pub source: Arc<str>,
    pub raw: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub level: Option<String>,
}

impl LogLine {
    pub fn new(
        source: Arc<str>,
        raw: String,
        timestamp: Option<DateTime<Utc>>,
        level: Option<String>,
    ) -> Self {
        Self {
            source,
            raw,
            timestamp,
            level,
        }
    }
}

pub struct LineReader {
    reader: BufReader<Box<dyn Read>>,
    buffer: String,
}

impl LineReader {
    fn new(path: &PathBuf) -> Result<Self> {
        let reader = build_reader(path)?;
        Ok(Self {
            reader,
            buffer: String::new(),
        })
    }
}

impl Iterator for LineReader {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.clear();

        match self.reader.read_line(&mut self.buffer) {
            Ok(0) => None,
            Ok(_) => {
                let line = self.buffer.trim().to_owned();
                Some(Ok(line))
            }
            Err(err) => Some(Err(err.into())),
        }
    }
}

pub struct LogLineStream {
    files: Vec<PathBuf>,
    next_file_idx: usize,
    current_reader: Option<LineReader>,
    date_format: String,
    source: Arc<str>,
    filters: LogFilters,
}

impl LogLineStream {
    fn new(
        path: PathBuf,
        input_from_stdin: bool,
        date_format: &str,
        filters: LogFilters,
    ) -> Result<Self> {
        let files = if path.is_dir() {
            list_files_recursive(&path)?
        } else {
            vec![path.clone()]
        };

        let source_label = if input_from_stdin {
            "stdin".to_string()
        } else {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string()
        };

        Ok(Self {
            files,
            next_file_idx: 0,
            current_reader: None,
            date_format: date_format.to_string(),
            source: Arc::from(source_label),
            filters,
        })
    }

    fn prepare_reader(&mut self) -> Result<bool> {
        while self.current_reader.is_none() {
            if self.next_file_idx >= self.files.len() {
                return Ok(false);
            }

            let path = self.files[self.next_file_idx].clone();
            self.next_file_idx += 1;

            let reader = read_lines(&path)
                .with_context(|| format!("Failed to read lines from '{}'", path.display()))?;

            self.current_reader = Some(reader);
        }

        Ok(true)
    }
}

impl Iterator for LogLineStream {
    type Item = Result<LogLine>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.prepare_reader() {
                Ok(true) => {}
                Ok(false) => return None,
                Err(err) => return Some(Err(err)),
            }

            let reader = self.current_reader.as_mut().expect("reader must exist");

            match reader.next() {
                Some(Ok(raw)) => {
                    let timestamp = extract_log_timestamp(raw.as_ref(), &self.date_format);
                    let level = detect_level(raw.as_ref());
                    let log_line = LogLine::new(self.source.clone(), raw, timestamp, level);

                    if self.filters.matches(&log_line) {
                        return Some(Ok(log_line));
                    }
                }
                Some(Err(err)) => return Some(Err(err)),
                None => {
                    self.current_reader = None;
                }
            }
        }
    }
}

#[derive(Args, Debug)]
pub struct CommonOpts {
    #[arg(
        long = "date-format",
        help = "Datetime format used at the beginning of the log line.",
        default_value = DEFAULT_DATE_FORMAT
    )]
    pub date_format: String,

    #[arg(
        long = "from-ts",
        help = "Lower bound datetime filter (uses --date-format)."
    )]
    pub from_ts: Option<String>,

    #[arg(
        long = "to-ts",
        help = "Upper bound datetime filter (uses --date-format)."
    )]
    pub to_ts: Option<String>,

    #[arg(
        long = "level",
        help = "Log levels to include (can be passed multiple times)."
    )]
    pub levels: Vec<String>,

    #[arg(long = "match", help = "Substring to match.")]
    pub substring_match: Option<String>,

    #[arg(long = "regex", help = "Regular expression to match.")]
    pub regex: Option<String>,
}

fn parse_datetime(ts: &str, format: &str, field_name: &str) -> Result<DateTime<Utc>> {
    // Try parsing with timezone first
    if let Ok(dt) = DateTime::parse_from_str(ts, format) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Fall back to naive datetime (no timezone) and assume UTC
    NaiveDateTime::parse_from_str(ts, format)
        .with_context(|| {
            format!(
                "Failed to parse {} '{}' with format '{}'",
                field_name, ts, format
            )
        })
        .map(|naive_dt| DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc))
}

pub fn build_filters(common: &CommonOpts) -> Result<LogFilters> {
    let from_ts = common
        .from_ts
        .as_ref()
        .map(|ts| parse_datetime(ts, &common.date_format, "--from-ts"))
        .transpose()?;

    let to_ts = common
        .to_ts
        .as_ref()
        .map(|ts| parse_datetime(ts, &common.date_format, "--to-ts"))
        .transpose()?;

    let compiled_regex = common
        .regex
        .as_ref()
        .map(|rgx| {
            regex::Regex::new(rgx)
                .with_context(|| format!("Failed to compile regex pattern '{}'", rgx))
        })
        .transpose()?;

    let normalized_levels = common
        .levels
        .iter()
        .map(|level| level.to_ascii_uppercase())
        .collect();

    Ok(LogFilters {
        from_ts,
        to_ts,
        levels: normalized_levels,
        substring_match: common.substring_match.clone(),
        regex: compiled_regex,
    })
}

pub fn list_files_recursive(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk(root, &mut out)?;
    Ok(out)
}

fn walk(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();

        if p.is_dir() {
            walk(&p, out)?; // recurse
        } else {
            out.push(p); // collect file
        }
    }
    Ok(())
}

pub fn read_lines(path: &PathBuf) -> Result<LineReader> {
    LineReader::new(path)
}

fn build_reader(path: &PathBuf) -> Result<BufReader<Box<dyn Read>>> {
    let is_gz = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("gz"))
        .unwrap_or(false);

    let file = File::open(path).with_context(|| format!("Failed to open '{}'", path.display()))?;

    let reader: Box<dyn Read> = if is_gz {
        Box::new(GzDecoder::new(file))
    } else {
        Box::new(file)
    };

    Ok(BufReader::new(reader))
}

pub(crate) fn extract_log_timestamp(line: &str, date_format: &str) -> Option<DateTime<Utc>> {
    // We try increasing slices up to a small prefix cap.
    let max_len = min(line.len(), 40);

    if max_len < 10 {
        return None;
    }

    // Iterate longest -> shortest so we return the most specific match.
    // Some chrono formats accept shorter prefixes by zero-filling, which
    // would silently collapse different timestamps to the same date.
    for end in (10..=max_len).rev() {
        let candidate = line[..end].trim();
        if let Ok(ts) = parse_datetime(candidate, date_format, "log") {
            return Some(ts);
        }
    }

    None
}

pub(crate) fn detect_level(line: &str) -> Option<String> {
    let upper = line.to_ascii_uppercase();

    for level in LEVELS {
        if upper.contains(level) {
            return Some(level.to_string());
        }
    }

    None
}

pub fn get_log_lines(
    path: PathBuf,
    input_from_stdin: bool,
    date_format: &str,
    filters: LogFilters,
) -> Result<LogLineStream> {
    LogLineStream::new(path, input_from_stdin, date_format, filters)
}

pub fn resolve_input_path(path: Option<PathBuf>) -> Result<(PathBuf, bool)> {
    match path {
        Some(p) => {
            if !p.exists() {
                return Err(anyhow::anyhow!("File does not exist: {}", p.display()));
            }
            Ok((p, false))
        }
        None => {
            eprintln!("Reading from stdin (Ctrl+D to end, Ctrl+C to cancel)...");
            let mut user_input = String::new();
            io::stdin().read_line(&mut user_input)?;
            let p = PathBuf::from(user_input.trim());
            if !p.exists() {
                return Err(anyhow::anyhow!("File does not exist: {}", p.display()));
            }
            Ok((p, true))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(raw: &str) -> LogLine {
        let ts = extract_log_timestamp(raw, DEFAULT_DATE_FORMAT);
        let lvl = detect_level(raw);
        LogLine::new(Arc::from("test"), raw.to_owned(), ts, lvl)
    }

    #[test]
    fn filters_match_level_substring_regex_and_ts() {
        let from = parse_datetime("2026-01-01 00:00:00", DEFAULT_DATE_FORMAT, "from").unwrap();
        let to = parse_datetime("2026-01-01 01:00:00", DEFAULT_DATE_FORMAT, "to").unwrap();
        let f = LogFilters {
            from_ts: Some(from),
            to_ts: Some(to),
            regex: Some(regex::Regex::new(r"boom").unwrap()),
            levels: vec!["ERROR".to_string()],
            substring_match: Some("boom".to_string()),
        };

        assert!(f.matches(&line("2026-01-01 00:30:00 ERROR boom happened")));
        assert!(!f.matches(&line("2026-01-01 00:30:00 INFO boom happened"))); // level mismatch
        assert!(!f.matches(&line("2026-01-01 00:30:00 ERROR all good"))); // no substring
        assert!(!f.matches(&line("2025-12-31 23:59:59 ERROR boom"))); // before range
        assert!(!f.matches(&line("no timestamp ERROR boom"))); // no ts when range required
    }

    #[test]
    fn levels_filter_normalizes_uppercase() {
        let f = LogFilters {
            from_ts: None,
            to_ts: None,
            regex: None,
            levels: vec!["ERROR".to_string()],
            substring_match: None,
        };
        assert!(f.matches(&line("2026-01-01 00:00:00 ERROR x")));
    }

    #[test]
    fn empty_filters_match_anything() {
        let f = LogFilters {
            from_ts: None,
            to_ts: None,
            regex: None,
            levels: vec![],
            substring_match: None,
        };
        assert!(f.matches(&line("anything goes")));
    }

    #[test]
    fn extract_log_timestamp_handles_short_and_long_lines() {
        assert!(extract_log_timestamp("short", DEFAULT_DATE_FORMAT).is_none());
        assert!(extract_log_timestamp("2026-01-01 00:00:00 hi", DEFAULT_DATE_FORMAT).is_some());
        // Past 40-char cap: still finds prefix.
        let long = "2026-01-01 00:00:00 ".to_string() + &"x".repeat(200);
        assert!(extract_log_timestamp(&long, DEFAULT_DATE_FORMAT).is_some());
    }

    #[test]
    fn extract_log_timestamp_picks_full_timestamp_not_date_only() {
        let ts1 = extract_log_timestamp("2026-01-01 00:00:00 INFO x", DEFAULT_DATE_FORMAT).unwrap();
        let ts2 = extract_log_timestamp("2026-01-01 00:00:01 INFO x", DEFAULT_DATE_FORMAT).unwrap();
        assert_ne!(ts1, ts2, "timestamps with different seconds must differ");
    }

    #[test]
    fn detect_level_finds_each_known_level() {
        assert_eq!(detect_level("blah INFO ok").as_deref(), Some("INFO"));
        assert_eq!(detect_level("info lowercase").as_deref(), Some("INFO"));
        assert_eq!(detect_level("nothing here"), None);
    }
}
