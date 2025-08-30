# Task number
0008
# What client asked
Print estimated remaining time in minutes and seconds and mention the target language when requesting the glossary.
# Technical solution
- compute ETA in milliseconds and format as minutes/seconds
- mention Brazilian Portuguese in the glossary system prompt
- add tests for ETA formatting and glossary prompt
# What changed
- translate loop logs human-readable ETA
- OpenAI glossary request names Brazilian Portuguese
- README documents minute/second ETA
# Notes
None
