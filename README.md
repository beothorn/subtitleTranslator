# subtitleTranslator

This is an experiment on automated coding without knowledge, don't use it.

## Install Rust and dependencies

1. Install Rust via [rustup](https://rustup.rs/).
2. Install `ffmpeg` so the extractor can run. On Ubuntu:

   ```bash
   sudo apt-get update && sudo apt-get install -y ffmpeg
   ```

## Build

```bash
cargo build --release
```

The binary will be at `target/release/subtra`.

## Usage

Extract English subtitles from a video file:

```bash
subtra --onlyextract foo.mkv
```

This will create `foo_en.srt` in the same directory.
