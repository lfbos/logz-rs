use clap::Args;

pub const DEFAULT_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

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
