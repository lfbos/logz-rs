use crate::cli::OutputFormat;
use crate::common::{CommonOpts, build_filters, get_log_lines, resolve_input_path};
use anyhow::Result;
use chrono::{DateTime, SecondsFormat, Utc};
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Default)]
struct Stats {
    total: usize,
    by_level: BTreeMap<String, usize>,
    by_source: BTreeMap<String, usize>,
    min_ts: Option<DateTime<Utc>>,
    max_ts: Option<DateTime<Utc>>,
}

/// Run the `stats` command.
pub fn run(path: Option<PathBuf>, format: &OutputFormat, common: &CommonOpts) -> Result<()> {
    let filters = build_filters(common)?;
    let (file_path, input_from_stdin) = resolve_input_path(path)?;

    let stream = get_log_lines(file_path, input_from_stdin, &common.date_format, filters)?;

    let mut stats = Stats::default();
    for log_line in stream {
        let line = log_line?;
        stats.total += 1;

        if let Some(level) = &line.level {
            *stats.by_level.entry(level.clone()).or_insert(0) += 1;
        }
        *stats
            .by_source
            .entry(line.source.as_ref().to_owned())
            .or_insert(0) += 1;

        if let Some(ts) = line.timestamp {
            stats.min_ts = Some(stats.min_ts.map_or(ts, |cur| cur.min(ts)));
            stats.max_ts = Some(stats.max_ts.map_or(ts, |cur| cur.max(ts)));
        }
    }

    let stdout = io::stdout();
    let mut w = stdout.lock();
    match format {
        OutputFormat::Json => write_json(&mut w, &stats)?,
        OutputFormat::Markdown => write_markdown(&mut w, &stats)?,
    }
    Ok(())
}

fn write_json<W: Write>(w: &mut W, s: &Stats) -> Result<()> {
    writeln!(w, "{{")?;
    writeln!(w, "  \"total\": {},", s.total)?;
    writeln!(w, "  \"min_ts\": {},", json_ts(&s.min_ts))?;
    writeln!(w, "  \"max_ts\": {},", json_ts(&s.max_ts))?;
    write_counts(w, "by_level", &s.by_level, true)?;
    write_counts(w, "by_source", &s.by_source, false)?;
    writeln!(w, "}}")?;
    Ok(())
}

fn write_counts<W: Write>(
    w: &mut W,
    name: &str,
    map: &BTreeMap<String, usize>,
    trailing_comma: bool,
) -> Result<()> {
    write!(w, "  \"{}\": {{", name)?;
    let mut first = true;
    for (k, v) in map {
        if !first {
            write!(w, ",")?;
        }
        first = false;
        write!(w, "\n    \"{}\": {}", json_escape(k), v)?;
    }
    if map.is_empty() {
        writeln!(w, "}}{}", if trailing_comma { "," } else { "" })?;
    } else {
        writeln!(w, "\n  }}{}", if trailing_comma { "," } else { "" })?;
    }
    Ok(())
}

fn json_ts(ts: &Option<DateTime<Utc>>) -> String {
    match ts {
        Some(t) => format!("\"{}\"", t.to_rfc3339_opts(SecondsFormat::Secs, true)),
        None => "null".to_string(),
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn write_markdown<W: Write>(w: &mut W, s: &Stats) -> Result<()> {
    writeln!(w, "# Log stats")?;
    writeln!(w)?;
    writeln!(w, "- **Total lines:** {}", s.total)?;
    writeln!(w, "- **Earliest timestamp:** {}", md_ts(&s.min_ts))?;
    writeln!(w, "- **Latest timestamp:** {}", md_ts(&s.max_ts))?;
    writeln!(w)?;

    md_table(w, "Level", &s.by_level)?;
    md_table(w, "Source", &s.by_source)?;
    Ok(())
}

fn md_table<W: Write>(w: &mut W, key: &str, map: &BTreeMap<String, usize>) -> Result<()> {
    writeln!(w, "## By {}", key.to_lowercase())?;
    writeln!(w)?;
    if map.is_empty() {
        writeln!(w, "_(none)_")?;
        writeln!(w)?;
        return Ok(());
    }
    writeln!(w, "| {} | Count |", key)?;
    writeln!(w, "| --- | --- |")?;
    for (k, v) in map {
        writeln!(w, "| {} | {} |", k, v)?;
    }
    writeln!(w)?;
    Ok(())
}

fn md_ts(ts: &Option<DateTime<Utc>>) -> String {
    ts.map(|t| t.to_rfc3339_opts(SecondsFormat::Secs, true))
        .unwrap_or_else(|| "—".to_string())
}
