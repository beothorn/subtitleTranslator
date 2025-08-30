# Task number
0012
# What client asked
Isolate prompts into files and allow tokens like $LANGUAGE with default "Brazilian Portuguese".
# Technical solution
- Added prompt templates under `crates/core/src/translate/prompts`.
- Replaced inline strings with templates loaded using `include_str!` and token substitution.
- Introduced `$LANGUAGE` token with a default of "Brazilian Portuguese".
# What changed
- Prompt files added for translation system, translation user, and glossary system.
- OpenAI translator now loads templates and replaces tokens.
# Notes
