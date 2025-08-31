//! Translation orchestration utilities.
//! This module wires subtitle parsing, OpenAI calls and output writing.

use crate::{srt, video};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, info, trace};

/// Default number of subtitle lines translated per batch.
pub const DEFAULT_BATCH_SIZE: usize = 50;

/// Translates a batch of lines with optional context (e.g., previous lines).
/// Represents a single line paired with its SRT index.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexedLine {
    /// SRT index associated with the line.
    pub index: u32,
    /// Text content of the line.
    pub text: String,
}

/// Translates a batch of lines with optional context (e.g., previous lines).
#[async_trait]
pub trait Translator: Send + Sync + Clone {
    /// Translate `lines` to the target locale preserving line boundaries.
    async fn translate_batch(
        &self,
        summary: &str,
        prev: &[String],
        lines: &[IndexedLine],
        target_locale: &str,
    ) -> Result<Vec<IndexedLine>>;

    /// Build a glossary and summary from a sample of lines.
    async fn build_glossary(&self, sample: &[String]) -> Result<String>;
}

pub mod openai;

#[derive(Clone)]
struct BatchJob {
    start: usize,
    prev: Vec<String>,
    lines: Vec<IndexedLine>,
}

/// Spawn a new asynchronous producer for a translation batch.
/// This function sends the translated lines back to the central consumer.
fn spawn_batch<T: Translator + Send + Sync + Clone + 'static>(
    job: BatchJob,
    tr: T,
    summary: String,
    tx: mpsc::Sender<(usize, Result<Vec<IndexedLine>>, u128)>,
) {
    tokio::spawn(async move {
        let begin = Instant::now();
        let res = tr
            .translate_batch(&summary, &job.prev, &job.lines, "pt-BR")
            .await;
        let elapsed = begin.elapsed().as_millis();
        let _ = tx.send((job.start, res, elapsed)).await;
    });
}

/// Process a video file or existing SRT by extracting or reading English
/// subtitles and translating them.
/// This function should output the translated SRT alongside the input file.
pub async fn process_file<T>(input: &Path, translator: T, batch_size: usize) -> Result<PathBuf>
where
    T: Translator + Send + Sync + Clone + 'static,
{
    trace!("process_file input={}", input.display());
    // Detect whether the input is already an SRT file so we skip extraction.
    let is_srt = input
        .extension()
        .map(|e| e.eq_ignore_ascii_case("srt"))
        .unwrap_or(false);
    let (content, temp) = if is_srt {
        info!("reading English subtitles");
        (fs::read_to_string(input)?, None)
    } else {
        info!("extracting English subtitles");
        let extracted = video::extract_english_subtitles(input)?;
        let temp = input.with_file_name(format!(
            "{}_temp_en.srt",
            input.file_stem().unwrap_or_default().to_string_lossy()
        ));
        fs::rename(&extracted, &temp)?;
        (fs::read_to_string(&temp)?, Some(temp))
    };
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
    let summary = translator.clone().build_glossary(&sample).await?;
    info!("glossary built");

    let partial_path = input.with_file_name(format!(
        "{}_partial_translation_pt_br",
        input.file_stem().unwrap_or_default().to_string_lossy()
    ));
    let (mut blocks, idx, _) = load_partial(&english_blocks, &partial_path)?;
    let total = blocks.len();
    if idx > 0 {
        let done = idx * 100 / total;
        info!("resuming at {done}%");
    }

    let (tx, mut rx) = mpsc::channel(english_blocks.len());
    let mut jobs: HashMap<usize, BatchJob> = HashMap::new();
    for start in (idx..english_blocks.len()).step_by(batch_size) {
        let end = (start + batch_size).min(english_blocks.len());
        let prev_start = start.saturating_sub(4);
        let prev: Vec<String> = english_blocks[prev_start..start]
            .iter()
            .map(|b| b.text.join("\n"))
            .collect();
        let lines: Vec<IndexedLine> = english_blocks[start..end]
            .iter()
            .map(|b| IndexedLine {
                index: b.index,
                text: b.text.join("\n"),
            })
            .collect();
        let job = BatchJob { start, prev, lines };
        jobs.insert(start, job.clone());
        spawn_batch(job, translator.clone(), summary.clone(), tx.clone());
    }

    let mut pending: BTreeMap<usize, (Vec<IndexedLine>, u128)> = BTreeMap::new();
    let mut next = idx;
    let mut last_ms: Option<u128> = None;
    while next < english_blocks.len() {
        let (start_idx, res, elapsed) = rx
            .recv()
            .await
            .ok_or_else(|| anyhow!("translation channel closed unexpectedly"))?;
        match res {
            Ok(translated) => {
                if let Some(job) = jobs.get(&start_idx) {
                    // In this branch we check if the translator returned the
                    // expected amount of lines and that each line actually
                    // changed. If something is off, we spawn the job again so
                    // the user never gets a partially translated file.
                    if translated.len() != job.lines.len()
                        || translated
                            .iter()
                            .zip(job.lines.iter())
                            .any(|(t, o)| t.text == o.text)
                    {
                        let end = start_idx + job.lines.len();
                        info!(
                            "retrying lines {}-{} due to incomplete translation",
                            start_idx + 1,
                            end
                        );
                        spawn_batch(job.clone(), translator.clone(), summary.clone(), tx.clone());
                        continue;
                    }
                }
                pending.insert(start_idx, (translated, elapsed));
            }
            Err(err) => {
                if let Some(job) = jobs.get(&start_idx) {
                    let end = start_idx + job.lines.len();
                    info!(
                        "retrying lines {}-{} after error: {}",
                        start_idx + 1,
                        end,
                        err
                    );
                    spawn_batch(job.clone(), translator.clone(), summary.clone(), tx.clone());
                }
                continue;
            }
        }
        while let Some((lines, elapsed)) = pending.remove(&next) {
            let end = next + lines.len();
            info!("translated lines {}-{} in {} ms", next + 1, end, elapsed);
            let mut map: HashMap<u32, String> =
                lines.into_iter().map(|l| (l.index, l.text)).collect();
            for block in blocks[next..end].iter_mut() {
                if let Some(text) = map.remove(&block.index) {
                    block.text = text.lines().map(|s| s.to_string()).collect();
                }
            }
            save_partial(&blocks, &partial_path)?;
            if let Some(prev) = last_ms {
                let remaining = blocks.len() - end;
                if remaining > 0 {
                    let estimate = estimate_remaining(prev, elapsed, remaining, batch_size);
                    info!("ETA: {}", format_eta(estimate));
                }
            }
            last_ms = Some(elapsed);
            next = end;
            let done = next * 100 / total;
            info!("completed {done}%");
        }
    }

    let out_path = if is_srt {
        input.with_file_name(format!(
            "{}_pt_br.srt",
            input.file_stem().unwrap_or_default().to_string_lossy()
        ))
    } else {
        input.with_extension("srt")
    };
    info!("writing output to {}", out_path.display());
    let out_content = srt::format(&blocks);
    fs::write(&out_path, out_content)?;
    if let Some(t) = temp {
        info!("removing temporary file");
        fs::remove_file(t)?;
    }
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

/// Format a duration in milliseconds as "X minute Y seconds".
/// This helper is used to log a readable ETA for the translation loop.
fn format_eta(ms: u128) -> String {
    trace!("format_eta ms={}", ms);
    let total_secs = ms / 1000;
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    if minutes > 0 {
        format!(
            "{} minute{} {} second{}",
            minutes,
            if minutes == 1 { "" } else { "s" },
            seconds,
            if seconds == 1 { "" } else { "s" }
        )
    } else {
        format!("{} second{}", seconds, if seconds == 1 { "" } else { "s" })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::fs;
    use std::sync::{Arc, Mutex};
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
        let ms = estimate_remaining(1000, 2000, 65, 50);
        assert_eq!(ms, 3000);
    }

    /// Ensure the ETA formatter outputs minutes and seconds.
    #[test]
    fn formats_eta() {
        assert_eq!(format_eta(110_000), "1 minute 50 seconds");
        assert_eq!(format_eta(45_000), "45 seconds");
    }

    /// Ensure we can translate an existing SRT file without extraction.
    #[tokio::test]
    async fn translates_existing_srt() {
        #[derive(Clone)]
        struct MockTr;
        #[async_trait]
        impl Translator for MockTr {
            /// Pretend to build a glossary by returning a dummy summary.
            async fn build_glossary(&self, _sample: &[String]) -> Result<String> {
                Ok("sum".into())
            }

            /// Translate by prefixing each line with `pt:` and keeping index.
            async fn translate_batch(
                &self,
                _summary: &str,
                _prev: &[String],
                lines: &[IndexedLine],
                _target_locale: &str,
            ) -> Result<Vec<IndexedLine>> {
                Ok(lines
                    .iter()
                    .map(|l| IndexedLine {
                        index: l.index,
                        text: format!("pt:{}", l.text),
                    })
                    .collect())
            }
        }

        let dir = tempdir().unwrap();
        let path = dir.path().join("orig.srt");
        fs::write(
            &path,
            "1\n00:00:00,000 --> 00:00:01,000\nhello\n\n2\n00:00:01,000 --> 00:00:02,000\nworld\n",
        )
        .unwrap();
        let out = process_file(&path, MockTr, 50).await.unwrap();
        assert_eq!(out, dir.path().join("orig_pt_br.srt"));
        let translated = fs::read_to_string(out).unwrap();
        assert!(translated.contains("pt:hello"));
        assert!(translated.contains("pt:world"));
    }

    /// Ensure we retry a batch when the translator errors once.
    #[tokio::test]
    async fn retries_failed_batch() {
        #[derive(Clone)]
        struct FlakyTr {
            attempts: Arc<Mutex<u32>>,
        }
        #[async_trait]
        impl Translator for FlakyTr {
            /// Pretend to build a glossary by returning a dummy summary.
            async fn build_glossary(&self, _sample: &[String]) -> Result<String> {
                Ok("sum".into())
            }

            /// Fail the first batch translation and succeed on subsequent tries.
            async fn translate_batch(
                &self,
                _summary: &str,
                _prev: &[String],
                lines: &[IndexedLine],
                _target_locale: &str,
            ) -> Result<Vec<IndexedLine>> {
                let mut lock = self.attempts.lock().unwrap();
                if *lock == 0 {
                    *lock += 1;
                    Err(anyhow!("boom"))
                } else {
                    Ok(lines
                        .iter()
                        .map(|l| IndexedLine {
                            index: l.index,
                            text: format!("pt:{}", l.text),
                        })
                        .collect())
                }
            }
        }

        let dir = tempdir().unwrap();
        let path = dir.path().join("orig.srt");
        fs::write(
            &path,
            "1\n00:00:00,000 --> 00:00:01,000\nhello\n\n",
        )
        .unwrap();
        let tr = FlakyTr {
            attempts: Arc::new(Mutex::new(0)),
        };
        let out = process_file(&path, tr, 50).await.unwrap();
        let translated = fs::read_to_string(out).unwrap();
        assert!(translated.contains("pt:hello"));
    }

    /// Ensure we retry when the translator returns the same lines without translating.
    #[tokio::test]
    async fn retries_untranslated_lines() {
        #[derive(Clone)]
        struct LazyTr {
            attempts: Arc<Mutex<u32>>,
        }
        #[async_trait]
        impl Translator for LazyTr {
            /// Pretend to build a glossary by returning a dummy summary.
            async fn build_glossary(&self, _sample: &[String]) -> Result<String> {
                Ok("sum".into())
            }

            /// First return the input unchanged, then prefix it with `pt:`.
            async fn translate_batch(
                &self,
                _summary: &str,
                _prev: &[String],
                lines: &[IndexedLine],
                _target_locale: &str,
            ) -> Result<Vec<IndexedLine>> {
                let mut lock = self.attempts.lock().unwrap();
                if *lock == 0 {
                    *lock += 1;
                    Ok(lines.to_vec())
                } else {
                    Ok(lines
                        .iter()
                        .map(|l| IndexedLine {
                            index: l.index,
                            text: format!("pt:{}", l.text),
                        })
                        .collect())
                }
            }
        }

        let dir = tempdir().unwrap();
        let path = dir.path().join("orig.srt");
        fs::write(
            &path,
            "1\n00:00:00,000 --> 00:00:01,000\nhi\n\n",
        )
        .unwrap();
        let tr = LazyTr {
            attempts: Arc::new(Mutex::new(0)),
        };
        let out = process_file(&path, tr, 50).await.unwrap();
        let translated = fs::read_to_string(out).unwrap();
        assert!(translated.contains("pt:hi"));
    }
}
