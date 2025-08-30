# 0006-progress-resume

# What client asked
Add progress percentage, extend OpenAI timeout to 90s, and persist partial translations to resume later.

# Technical solution
Calculate completion after each batch and log at info level; serialize `SrtBlock` list to JSON in a `*_partial_translation_pt_br` file; load it to resume and adjust OpenAI client timeout.

# What changed
- log completion percentage and resume status during translation
- save and load partial translations alongside videos
- increase OpenAI request timeout to 90 seconds

# Notes
Handles resuming from `*_partial_translation_pt_br` files.
