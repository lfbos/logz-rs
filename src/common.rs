use std::{
    cmp::min,
    fs::{self, File},
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use clap::Args;
use flate2::read::GzDecoder;

pub const DEFAULT_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const LEVELS: [&str; 6] = ["DEBUG", "INFO", "WARN", "WARNING", "ERROR", "CRITICAL"];

pub struct LogFilters {
    pub from_ts: Option<DateTime<Utc>>,
    pub to_ts: Option<DateTime<Utc>>,
    pub regex: Option<regex::Regex>,
    pub levels: Vec<String>,
    pub substring_match: Option<String>,
}

#[derive(Debug)]
pub struct LogLine {
    source: Arc<str>,
    raw: String,
    timestamp: Option<DateTime<Utc>>,
    level: Option<String>,
}

impl LogLine {
    fn new(
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

    Ok(LogFilters {
        from_ts,
        to_ts,
        levels: common.levels.clone(),
        substring_match: common.substring_match.clone(),
        regex: compiled_regex,
    })
}

pub fn list_files_recursive(root: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk(root.as_path(), &mut out)?;
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

fn extract_log_timestamp(line: &str, date_format: &str) -> Option<DateTime<Utc>> {
    // We try increasing slices up to a small prefix cap.
    let max_len = min(line.len(), 40);

    if max_len < 10 {
        return None;
    }

    for end in 10..=max_len {
        let candidate = line[..end].trim();
        if let Ok(ts) = parse_datetime(candidate, date_format, "log") {
            return Some(ts);
        }
    }

    None
}

fn detect_level(line: &str) -> Option<String> {
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
) -> Result<Vec<LogLine>> {
    // Get all the files available
    let mut all_files: Vec<PathBuf> = Vec::new();
    if path.is_dir() {
        all_files = list_files_recursive(&path)?;
    } else {
        all_files.push(path.clone());
    }

    let source_label = if input_from_stdin {
        "stdin".to_string()
    } else {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    };
    let source: Arc<str> = Arc::from(source_label);

    let mut log_lines: Vec<LogLine> = Vec::new();

    for file in all_files {
        let reader = read_lines(&file)
            .with_context(|| format!("Failed to read lines from '{}'", file.display()))?;

        for raw in reader {
            let raw = raw?;
            let timestamp: Option<DateTime<Utc>> = extract_log_timestamp(raw.as_ref(), date_format);
            let level = detect_level(raw.as_ref());

            log_lines.push(LogLine::new(source.clone(), raw, timestamp, level));
        }
    }

    Ok(log_lines)
}
