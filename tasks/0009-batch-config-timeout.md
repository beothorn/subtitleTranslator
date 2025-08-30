# Task number
0009
# What client asked
Increase translation batch to 50 lines, make batch size configurable via `--batch-size`, and retry OpenAI requests on timeouts instead of exiting.
# Technical solution
Exposed `DEFAULT_BATCH_SIZE` in translate module and updated `process_file` to accept a `batch_size` parameter. Wired CLI `--batch-size` option to pass the value. Added retry loop in OpenAI client for timeouts and allowed request timeout override via `OPENAI_TIMEOUT_SECS`.
# What changed
- configurable batch size with default 50
- OpenAI requests retry after timeouts
- README documents `--batch-size` and retry behavior
# Notes

