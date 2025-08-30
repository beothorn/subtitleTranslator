# Task number
0003
# What client asked
Translate extracted subtitles to Brazilian Portuguese using OpenAI.
# Technical solution
Add SRT parsing and OpenAI translator that processes subtitles in batches.
Introduce a CLI path to extract subtitles, build a glossary from the first 15 lines, translate 10 entries at a time with context from the previous four entries, and write the final `.srt` file.
# What changed
- Added SRT parser and formatter.
- Implemented OpenAI translator and translation pipeline.
- Extended CLI to call translator when not in `--onlyextract` mode.
- Documented translation feature and usage in README.
# Notes
