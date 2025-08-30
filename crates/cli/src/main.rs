//! Binary entry point for the subtitle extractor.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use subtra_core::translate::{openai::OpenAiTranslator, process_file, DEFAULT_BATCH_SIZE};
use subtra_core::video::extract_english_subtitles;
use tracing_subscriber::EnvFilter;

/// Command line options for the binary.
#[derive(Parser)]
struct Cli {
    /// When true we only extract the English subtitles from the input file.
    #[arg(long)]
    onlyextract: bool,

    /// Enable verbose debug and trace logs.
    #[arg(long)]
    debug: bool,

    /// Number of subtitle lines to translate per batch.
    #[arg(long, default_value_t = DEFAULT_BATCH_SIZE)]
    batch_size: usize,

    /// Path to the video or SRT file we want to process.
    input: PathBuf,
}

/// Application entry point which parses CLI args and performs actions.
/// This function should initialize logging and delegate to the core library.
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let filter = if cli.debug {
        EnvFilter::default()
            .add_directive("subtra=trace".parse().unwrap())
            .add_directive("subtra_core=trace".parse().unwrap())
            .add_directive("info".parse().unwrap())
    } else {
        EnvFilter::default()
            .add_directive("subtra=info".parse().unwrap())
            .add_directive("subtra_core=info".parse().unwrap())
            .add_directive("warn".parse().unwrap())
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();
    if cli.onlyextract {
        extract_english_subtitles(&cli.input)?;
    } else {
        let translator = OpenAiTranslator::new()?;
        process_file(&cli.input, translator, cli.batch_size).await?;
    }
    Ok(())
}
