# Task number
0013
# What client asked
Send only JSON with the index to the translator to prevent confusion with SRT formatting.
# Technical solution
Build JSON payloads containing each line's index and text, adjust prompts to reference the JSON format.
# What changed
- Use JSON objects for translation batches in the OpenAI translator.
- Update translation user prompt to describe the JSON input and output.
# Notes
None
