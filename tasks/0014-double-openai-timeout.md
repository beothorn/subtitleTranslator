# Task number
0014
# What client asked
Double the OpenAI request timeout to reduce frequent timeouts.
# Technical solution
Increase default OPENAI_TIMEOUT_SECS from 90 to 180 seconds in `OpenAiTranslator::new`.
# What changed
- doubled OpenAI request timeout default to 180 seconds
# Notes
