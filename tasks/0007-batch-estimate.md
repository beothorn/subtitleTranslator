# 0007
# What client asked
increase batch from 10 to 30 lines and log estimated remaining time based on recent calls
# Technical solution
- set translation batch size constant to 30
- track last two request durations to estimate time left
# What changed
- translation loop now processes 30-line chunks and logs ETA
- added helper for estimating remaining time with tests
- README documents batch size and ETA logging
# Notes
