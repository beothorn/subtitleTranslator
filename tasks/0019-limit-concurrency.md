# 0019-limit-concurrency
# What client asked
Limit OpenAI requests to 4 threads at a time to avoid hitting rate limits.
# Technical solution
Add a semaphore in the translation orchestrator to cap concurrent batches at four and test that limit.
# What changed
- Introduced MAX_CONCURRENT_BATCHES constant and semaphore to throttle batch spawning
- Added regression test verifying at most four concurrent batches run
# Notes
