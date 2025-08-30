# Task number
0004
# What client asked
delete _temp_en.srt when translation finishes, add --debug option for logs, and log OpenAI requests and responses to debug timeouts
# Technical solution
Remove temp file after successful translation, add CLI flag to set logging level to TRACE, and instrument OpenAI calls with request/response logs and configurable base URL
# What changed
- remove temporary subtitle after writing final output
- add --debug flag to enable trace logging
- log OpenAI requests/responses with timeout info and support custom base URL
# Notes

