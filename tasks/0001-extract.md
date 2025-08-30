# Task number
0001
# What client asked
Add basic command that extracts the English subtitle and builds binary.
# Technical solution
Create Rust workspace with core library for video extraction and CLI binary using clap. Implement `extract_english_subtitles` using ffmpeg command and add README instructions.
# What changed
- Added workspace with core and cli crates.
- Implemented English subtitle extraction.
- Added CLI command `--onlyextract`.
- Documented build and usage instructions.
# Notes

