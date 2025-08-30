# Task number
0002
# What client asked
Fix mp4 extraction failing when multiple English subtitle streams exist.
# Technical solution
Use ffprobe to list subtitle streams, select the best English closed-caption track with a simple heuristic, then map that index when calling ffmpeg.
# What changed
- Probed subtitle streams with ffprobe and chose the most relevant English track.
- Mapped the selected stream index when extracting subtitles.
# Notes
