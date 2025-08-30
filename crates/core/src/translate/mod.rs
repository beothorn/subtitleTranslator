//! Translation orchestration utilities.
//! This module wires subtitle parsing, OpenAI calls and output writing.

use crate::{srt, video};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, trace};

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
    let mut blocks = srt::parse(&content)?;

    let mut sample = Vec::new();
    for block in &blocks {
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

    let mut history: Vec<String> = Vec::new();
    let mut idx = 0;
    while idx < blocks.len() {
        let end = (idx + 10).min(blocks.len());
        info!(
            "translating lines {}-{} of {}",
            idx + 1,
            end,
            blocks.len()
        );
        let chunk = &mut blocks[idx..end];
        let english: Vec<String> = chunk.iter().map(|b| b.text.join("\n")).collect();
        let start = std::time::Instant::now();
        let translated = translator.translate_batch(&summary, &history, &english, "pt-BR")?;
        info!(
            "translated lines {}-{} in {} ms",
            idx + 1,
            end,
            start.elapsed().as_millis()
        );
        for (block, text) in chunk.iter_mut().zip(translated.into_iter()) {
            block.text = text.lines().map(|s| s.to_string()).collect();
        }
        history.extend(english);
        if history.len() > 4 {
            history = history[history.len() - 4..].to_vec();
        }
        idx = end;
    }

    let out_path = input.with_extension("srt");
    info!("writing output to {}", out_path.display());
    let out_content = srt::format(&blocks);
    fs::write(&out_path, out_content)?;
    info!("removing temporary file");
    fs::remove_file(&temp)?;
    info!("wrote {}", out_path.display());
    Ok(out_path)
}
