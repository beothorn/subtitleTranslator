# Task number
0002
# What client asked
Fix mp4 extraction failing when multiple English subtitle streams exist.
# Technical solution
Use ffprobe to list subtitle streams, select the best English closed-caption track with a simple heuristic, then map the subtitle stream number when calling ffmpeg and transcode it to SRT.
# What changed
- Probed subtitle streams with ffprobe and chose the most relevant English track.
- Mapped the selected subtitle stream number and forced SRT output during extraction.
# Notes
