//! This module is responsible for SRT parsing and integrity checks.
//! It exposes helpers to read and write SRT blocks while preserving timing.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Represents a single SRT block (index, time range, text lines).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SrtBlock {
    pub index: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: Vec<String>,
}

/// Parse SRT text into a list of blocks.
/// This function should read indices, times and text lines preserving order.
pub fn parse(input: &str) -> Result<Vec<SrtBlock>> {
    let mut blocks = Vec::new();
    let mut lines = input.lines();
    loop {
        let index_line = match lines.next() {
            Some(l) if !l.trim().is_empty() => l.trim(),
            Some(_) => continue,
            None => break,
        };
        let index: u32 = index_line.parse()?;
        let time_line = lines.next().ok_or_else(|| anyhow!("missing time"))?;
        let (start_ms, end_ms) = parse_times(time_line)?;
        let mut text = Vec::new();
        for line in lines.by_ref() {
            if line.trim().is_empty() {
                break;
            }
            text.push(line.to_string());
        }
        blocks.push(SrtBlock {
            index,
            start_ms,
            end_ms,
            text,
        });
    }
    Ok(blocks)
}

/// Format SRT blocks back to text.
/// The way this works is by writing each block sequentially with blank lines.
pub fn format(blocks: &[SrtBlock]) -> String {
    let mut out = String::new();
    for block in blocks {
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            block.index,
            format_time(block.start_ms),
            format_time(block.end_ms),
            block.text.join("\n")
        ));
    }
    out
}

/// Parse a time range like `00:00:01,000 --> 00:00:02,000` to milliseconds.
fn parse_times(line: &str) -> Result<(u64, u64)> {
    let mut parts = line.split(" --> ");
    let start = parts.next().ok_or_else(|| anyhow!("no start"))?;
    let end = parts.next().ok_or_else(|| anyhow!("no end"))?;
    Ok((parse_time(start)?, parse_time(end)?))
}

/// Parse `HH:MM:SS,mmm` into milliseconds.
fn parse_time(t: &str) -> Result<u64> {
    let parts: Vec<&str> = t.split([':', ',']).collect();
    if parts.len() != 4 {
        return Err(anyhow!("bad time"));
    }
    let h: u64 = parts[0].parse()?;
    let m: u64 = parts[1].parse()?;
    let s: u64 = parts[2].parse()?;
    let ms: u64 = parts[3].parse()?;
    Ok(((h * 60 + m) * 60 + s) * 1000 + ms)
}

/// Format milliseconds back to `HH:MM:SS,mmm`.
fn format_time(ms: u64) -> String {
    let h = ms / 3_600_000;
    let m = (ms % 3_600_000) / 60_000;
    let s = (ms % 60_000) / 1000;
    let ms = ms % 1000;
    format!("{h:02}:{m:02}:{s:02},{ms:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_srt() {
        let input = "1\n00:00:00,000 --> 00:00:01,000\nHello\n\n";
        let blocks = parse(input).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, vec!["Hello".to_string()]);
        let out = format(&blocks);
        assert_eq!(input, out);
    }
}
