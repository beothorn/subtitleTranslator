//! Binary entry point for the subtitle extractor.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use subtra_core::translate::{openai::OpenAiTranslator, process_file};
use subtra_core::video::extract_english_subtitles;

/// Command line options for the binary.
#[derive(Parser)]
struct Cli {
    /// When true we only extract the English subtitles from the input file.
    #[arg(long)]
    onlyextract: bool,

    /// Path to the video file we want to process.
    input: PathBuf,
}

/// Application entry point which parses CLI args and performs actions.
/// This function should initialize logging and delegate to the core library.
fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    if cli.onlyextract {
        extract_english_subtitles(&cli.input)?;
    } else {
        let translator = OpenAiTranslator::new()?;
        process_file(&cli.input, &translator)?;
    }
    Ok(())
}
