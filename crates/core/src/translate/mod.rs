//! Translation orchestration utilities.
//! This module wires subtitle parsing, OpenAI calls and output writing.

use crate::{srt, video};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, trace};

const BATCH_SIZE: usize = 30;

/// Translates a batch of lines with optional context (e.g., previous lines).
pub trait Translator {
    /// Translate `lines` to the target locale preserving line boundaries.
    fn translate_batch(
        &self,
        summary: &str,
        prev: &[String],
        lines: &[String],
        target_locale: &str,
    ) -> Result<Vec<String>>;

    /// Build a glossary and summary from a sample of lines.
    fn build_glossary(&self, sample: &[String]) -> Result<String>;
}

pub mod openai;

/// Process a video file by extracting English subtitles and translating them.
/// This function should output the translated SRT alongside the input file.
pub fn process_file(input: &Path, translator: &impl Translator) -> Result<PathBuf> {
    trace!("process_file input={}", input.display());
    info!("extracting English subtitles");
    let extracted = video::extract_english_subtitles(input)?;
    let temp = input.with_file_name(format!(
        "{}_temp_en.srt",
        input.file_stem().unwrap_or_default().to_string_lossy()
    ));
    fs::rename(&extracted, &temp)?;
    let content = fs::read_to_string(&temp)?;
    let english_blocks = srt::parse(&content)?;

    let mut sample = Vec::new();
    for block in &english_blocks {
        for line in &block.text {
            if sample.len() >= 15 {
                break;
            }
            sample.push(line.clone());
        }
        if sample.len() >= 15 {
            break;
        }
    }
    info!("building glossary from sample");
    let summary = translator.build_glossary(&sample)?;
    info!("glossary built");

    let partial_path = input.with_file_name(format!(
        "{}_partial_translation_pt_br",
        input.file_stem().unwrap_or_default().to_string_lossy()
    ));
    let (mut blocks, mut idx, mut history) = load_partial(&english_blocks, &partial_path)?;
    let total = blocks.len();
    if idx > 0 {
        let done = idx * 100 / total;
        info!("resuming at {done}%");
    }

    let mut last_ms: Option<u128> = None;
    while idx < blocks.len() {
        let end = (idx + BATCH_SIZE).min(blocks.len());
        let progress = end * 100 / total;
        info!(
            "translating lines {}-{} of {} ({}%)",
            idx + 1,
            end,
            total,
            progress
        );
        let chunk = &mut blocks[idx..end];
        let english: Vec<String> = english_blocks[idx..end]
            .iter()
            .map(|b| b.text.join("\n"))
            .collect();
        let start = std::time::Instant::now();
        let translated =
            translator.translate_batch(&summary, &history, &english, "pt-BR")?;
        let elapsed = start.elapsed().as_millis();
        info!(
            "translated lines {}-{} in {} ms",
            idx + 1,
            end,
            elapsed
        );
        for (block, text) in chunk.iter_mut().zip(translated.into_iter()) {
            block.text = text.lines().map(|s| s.to_string()).collect();
        }
        history.extend(english);
        if history.len() > 4 {
            history = history[history.len() - 4..].to_vec();
        }
        idx = end;
        save_partial(&blocks, &partial_path)?;
        if let Some(prev) = last_ms {
            let remaining = blocks.len() - idx;
            let estimate = estimate_remaining(prev, elapsed, remaining, BATCH_SIZE);
            info!("estimated {} s remaining", estimate / 1000);
        }
        last_ms = Some(elapsed);
        let done = idx * 100 / total;
        info!("completed {done}%");
    }

    let out_path = input.with_extension("srt");
    info!("writing output to {}", out_path.display());
    let out_content = srt::format(&blocks);
    fs::write(&out_path, out_content)?;
    info!("removing temporary file");
    fs::remove_file(&temp)?;
    if partial_path.exists() {
        info!("removing partial translation {}", partial_path.display());
        fs::remove_file(&partial_path)?;
    }
    info!("wrote {}", out_path.display());
    Ok(out_path)
}

/// Load an existing partial translation if available.
/// This function should read a JSON file of blocks and compute the resume index and history.
fn load_partial(
    original: &[srt::SrtBlock],
    path: &Path,
) -> Result<(Vec<srt::SrtBlock>, usize, Vec<String>)> {
    trace!("load_partial path={}", path.display());
    if !path.exists() {
        return Ok((original.to_vec(), 0, Vec::new()));
    }
    let text = fs::read_to_string(path)?;
    let blocks: Vec<srt::SrtBlock> = serde_json::from_str(&text)?;
    let mut idx = 0;
    while idx < blocks.len() && blocks[idx].text != original[idx].text {
        idx += 1;
    }
    let start = idx.saturating_sub(4);
    let history = original[start..idx]
        .iter()
        .map(|b| b.text.join("\n"))
        .collect();
    Ok((blocks, idx, history))
}

/// Save the current translation progress to disk.
/// The way this works is by serializing the blocks to JSON for later resumption.
fn save_partial(blocks: &[srt::SrtBlock], path: &Path) -> Result<()> {
    trace!("save_partial path={}", path.display());
    let text = serde_json::to_string(blocks)?;
    fs::write(path, text)?;
    debug!("saved partial translation to {}", path.display());
    Ok(())
}

/// Estimate remaining time in milliseconds for the translation.
/// The way this works is by averaging `prev_ms` and `curr_ms` and
/// multiplying by the number of batches left.
fn estimate_remaining(prev_ms: u128, curr_ms: u128, remaining: usize, batch: usize) -> u128 {
    trace!(
        "estimate_remaining prev_ms={} curr_ms={} remaining={} batch={}",
        prev_ms,
        curr_ms,
        remaining,
        batch
    );
    let avg = (prev_ms + curr_ms) / 2;
    let batches = (remaining + batch - 1) / batch;
    avg * batches as u128
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Ensure we can load a partial file and resume from the correct index.
    #[test]
    fn resumes_from_partial() {
        let blocks = vec![
            srt::SrtBlock {
                index: 1,
                start_ms: 0,
                end_ms: 1000,
                text: vec!["a".into()],
            },
            srt::SrtBlock {
                index: 2,
                start_ms: 1000,
                end_ms: 2000,
                text: vec!["b".into()],
            },
        ];
        let dir = tempdir().unwrap();
        let partial = dir.path().join("video_partial_translation_pt_br");
        let mut translated = blocks.clone();
        translated[0].text = vec!["pt:a".into()];
        save_partial(&translated, &partial).unwrap();
        let (loaded, idx, history) = load_partial(&blocks, &partial).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(history, vec!["a".to_string()]);
        assert_eq!(loaded[0].text, vec!["pt:a".to_string()]);
    }

    /// Verify the time estimation uses the average of the last two calls and remaining batches.
    #[test]
    fn estimates_remaining_time() {
        let ms = estimate_remaining(1000, 2000, 65, 30);
        assert_eq!(ms, 4500);
    }
}
