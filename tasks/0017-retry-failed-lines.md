# Task number
0017
# What client asked
Retry failed translation batches instead of erroring.
# Technical solution
Add a batch map and respawn failed batches until all lines are translated.
# What changed
- Retry mechanism in translation pipeline
- Added unit test for retrying failed batch
# Notes
