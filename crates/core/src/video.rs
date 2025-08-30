//! Video helpers for working with subtitles.

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::trace;

/// Build the ffmpeg arguments to extract English subtitles and the output path.
/// The way this works is by mapping the English subtitle track and copying it to
/// an SRT file with `_en` suffix.
pub fn ffmpeg_extract_args(input: &Path) -> (PathBuf, Vec<String>) {
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
        "0:s:m:language:eng".to_string(),
        "-c:s".to_string(),
        "copy".to_string(),
        out.display().to_string(),
    ];
    (out, args)
}

/// Extract English subtitles from `path` using ffmpeg.
/// This function should call ffmpeg with the correct args and return the output path.
pub fn extract_english_subtitles(path: &Path) -> Result<PathBuf> {
    trace!(
        "extract_english_subtitles(path={}): invoking ffmpeg",
        path.display()
    );
    let (out, args) = ffmpeg_extract_args(path);
    let status = Command::new("ffmpeg").args(&args).status()?;
    if !status.success() {
        return Err(anyhow!("ffmpeg failed"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn builds_expected_ffmpeg_args() {
        let input = Path::new("foo.mkv");
        let (out, args) = ffmpeg_extract_args(input);
        assert_eq!(out, PathBuf::from("foo_en.srt"));
        assert_eq!(
            args,
            vec![
                "-i",
                "foo.mkv",
                "-map",
                "0:s:m:language:eng",
                "-c:s",
                "copy",
                "foo_en.srt"
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
        );
    }
}
