//! Video helpers for working with subtitles.

use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::trace;

/// Represents a subtitle stream returned by ffprobe.
/// This type holds optional language and title tags.
#[derive(Debug, Deserialize)]
struct Stream {
    #[serde(default)]
    tags: Tags,
}

/// Captures the language and title tags for a stream.
/// ffprobe may omit these fields, so they are optional.
#[derive(Debug, Default, Deserialize)]
struct Tags {
    language: Option<String>,
    title: Option<String>,
}

/// Build the ffmpeg arguments to extract a subtitle track and the output path.
/// This delegates the choice of stream to the caller via `stream_index`.
pub fn ffmpeg_extract_args(input: &Path, stream_index: usize) -> (PathBuf, Vec<String>) {
    let stem = input
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let out = input.with_file_name(format!("{}_en.srt", stem));
    let args = vec![
        "-i".to_string(),
        input.display().to_string(),
        "-map".to_string(),
        format!("0:s:{}", stream_index),
        "-c:s".to_string(),
        "srt".to_string(),
        out.display().to_string(),
    ];
    (out, args)
}

/// Extract English subtitles from `path` using ffmpeg.
/// This probes available subtitle streams, picks the best English track and
/// then calls ffmpeg to copy it to an SRT file.
pub fn extract_english_subtitles(path: &Path) -> Result<PathBuf> {
    trace!(
        "extract_english_subtitles(path={}): invoking ffmpeg",
        path.display()
    );
    let index = pick_subtitle_index(path)?;
    let (out, args) = ffmpeg_extract_args(path, index);
    let status = Command::new("ffmpeg").args(&args).status()?;
    if !status.success() {
        return Err(anyhow!("ffmpeg failed"));
    }
    Ok(out)
}

/// Decide which English subtitle stream to extract.
/// The way this works is by scoring English streams based on their title
/// and picking the one that looks most like a closed caption track.
fn best_english_stream(streams: &[Stream]) -> Option<usize> {
    let mut best: Option<(usize, i32)> = None;
    for (i, stream) in streams.iter().enumerate() {
        let lang = stream
            .tags
            .language
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case("eng"))
            .unwrap_or(false);
        if !lang {
            continue;
        }
        let title = stream.tags.title.as_deref().unwrap_or("").to_lowercase();
        let score = if title.contains("cc") || title.contains("sdh") || title.contains("caption") {
            2
        } else if title.contains("sub") {
            1
        } else {
            0
        };
        match best {
            Some((_, best_score)) if score <= best_score => {}
            _ => best = Some((i, score)),
        }
    }
    best.map(|(idx, _)| idx)
}

/// Probe subtitle streams with ffprobe and pick the best English track.
/// It returns the stream index to map with ffmpeg.
fn pick_subtitle_index(path: &Path) -> Result<usize> {
    trace!(
        "pick_subtitle_index(path={}): listing subtitle streams",
        path.display()
    );
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "s",
            "-show_entries",
            "stream_tags=language,title",
            "-of",
            "json",
            path.to_string_lossy().as_ref(),
        ])
        .output()?;
    if !output.status.success() {
        return Err(anyhow!("ffprobe failed"));
    }
    #[derive(Deserialize)]
    struct Streams {
        streams: Vec<Stream>,
    }
    let data: Streams = serde_json::from_slice(&output.stdout)?;
    if let Some(idx) = best_english_stream(&data.streams) {
        Ok(idx)
    } else {
        Err(anyhow!("no english subtitles found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn builds_expected_ffmpeg_args() {
        let input = Path::new("foo.mkv");
        let (out, args) = ffmpeg_extract_args(input, 3);
        assert_eq!(out, PathBuf::from("foo_en.srt"));
        let expected = [
            "-i",
            "foo.mkv",
            "-map",
            "0:s:3",
            "-c:s",
            "srt",
            "foo_en.srt",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
        assert_eq!(args, expected);
    }

    #[test]
    fn picks_cc_stream_over_plain() {
        let streams = vec![
            Stream {
                tags: Tags {
                    language: Some("eng".to_string()),
                    title: Some("English".to_string()),
                },
            },
            Stream {
                tags: Tags {
                    language: Some("eng".to_string()),
                    title: Some("English CC".to_string()),
                },
            },
        ];
        assert_eq!(best_english_stream(&streams), Some(1));
    }
}
