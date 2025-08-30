# Task number
0011
# What client asked
Fix misalignment of translated lines by including SRT index in prompts and returning JSON with indexes.
# Technical solution
Introduce an `IndexedLine` struct and update the translation pipeline to send lines with their indices and parse OpenAI responses containing `translatedLines` objects keyed by index.
# What changed
- Added `IndexedLine` struct and updated `Translator` trait.
- Modified OpenAI translator to request/parse indexed JSON and include an example in the prompt.
- Updated processing logic to map translations by index.
- Adjusted tests and mock translator implementation.
# Notes
None

