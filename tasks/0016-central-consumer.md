# 0016-central-consumer

# What client asked
Fix async translation so all lines are processed and logging/ETA are correct. Centralize translation writes.

# Technical solution
Spawn translation producers that send results over an mpsc channel to a single consumer which updates blocks, tracks progress and ETA, and ensures every line is translated.

# What changed
- Used an mpsc channel with a central consumer to apply translations in order
- Simplified dependencies and added tokio to core crate
- Added a check to ensure 100% of lines are translated before writing output

# Notes

