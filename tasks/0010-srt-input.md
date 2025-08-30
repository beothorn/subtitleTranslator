# Task number
0010
# What client asked
When an srt file is passed as parameter instead of a video file, it should translate it, the same way it would translate the one that was extracted from the video file. The file name should be name_of_original_pt_br.srt
# Technical solution
- Detect `.srt` inputs and skip subtitle extraction.
- Translate the provided subtitles directly and write `<stem>_pt_br.srt`.
- Added test with a mock translator to cover SRT inputs.
- Updated CLI docs and README to document SRT support.
# What changed
- `process_file` now handles SRT files and names output with `_pt_br`.
- CLI help mentions video or SRT inputs.
- README explains translating SRT files.
- Added regression test and task record.
# Notes
