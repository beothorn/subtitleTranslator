# Task number
0018-requeue-untranslated

# What client asked
If some lines were not translated, try again and never error.

# Technical solution
Detect when a translation batch returns lines identical to the input and requeue that batch until it changes. Remove the final untranslated-lines check.

# What changed
- Requeue batches whose translation is unchanged
- Drop post-loop untranslated-lines error
- Add a regression test for unchanged translations

# Notes
Retries run until a different translation is returned.
