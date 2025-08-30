# Task number
0015
# What client asked
Make the translation async and parallelize batches while keeping output order.
# Technical solution
Converted translation trait and OpenAI implementation to async using `async-trait` and `reqwest` async client. Reworked `process_file` to spawn all batch translations concurrently with `FuturesUnordered`, collecting results in order using a `BTreeMap`. Updated CLI to run inside a Tokio runtime.
# What changed
- Added async translation infrastructure and parallel batch processing.
- Switched to async reqwest client and futures utilities.
- CLI now uses Tokio runtime and awaits `process_file`.
# Notes
None.
