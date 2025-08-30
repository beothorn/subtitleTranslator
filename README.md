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

Translate a video to Brazilian Portuguese:

```bash
export OPENAI_API_KEY=...  # your OpenAI key
subtra /path/to/video.mkv
```

This creates `/path/to/video.srt` with Portuguese subtitles.
Progress is logged as a percentage and the tool saves a partial translation to
`/path/to/video_partial_translation_pt_br`. If interrupted, re-running the same
command resumes from where it left off.

Extract only the English subtitles from a video file:

```bash
subtra --onlyextract foo.mkv
```

This will create `foo_en.srt` in the same directory.

Show detailed logs for debugging:

```bash
subtra --debug video.mkv
```
This prints verbose progress messages for the translation pipeline while keeping third-party noise to a minimum.
