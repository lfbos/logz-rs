use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use clap::Args;

pub const DEFAULT_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub struct LogFilters {
    pub from_ts: Option<DateTime<Utc>>,
    pub to_ts: Option<DateTime<Utc>>,
    pub regex: Option<regex::Regex>,
    pub levels: Vec<String>,
    pub substring_match: Option<String>,
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
        .with_context(|| format!("Failed to parse {} '{}' with format '{}'", field_name, ts, format))
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
