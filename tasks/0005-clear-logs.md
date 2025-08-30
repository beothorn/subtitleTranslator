# Task number
0005
# What client asked
Improve debug output so the user knows when the app is waiting and reduce noisy traces.
# Technical solution
Added human-readable info logs for each translation step, filtered out external crate traces, and tightened OpenAI timeouts.
# What changed
- Filtered tracing to project modules only when using --debug
- Added progress logs and clearer OpenAI request timing
- Shortened OpenAI timeouts for faster failures
# Notes

