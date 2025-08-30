# Task number
0002
# What client asked
Fix mp4 extraction failing when multiple English subtitle streams exist.
# Technical solution
Limit ffmpeg mapping to the first English subtitle track to avoid multiple-stream errors when writing SRT output.
# What changed
- Updated ffmpeg arguments to map only the first English subtitle stream.
# Notes
