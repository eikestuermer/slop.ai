# sample-interview

A tiny fixture project. Two takes from a fictional founder interview plus
one B-roll clip. Used by integration tests and as a "hello world" for
contributors.

The actual media files are not bundled. The `assets.json` file lists their
expected URIs and durations; we generate synthetic transcripts and shot
lists at test time so the rest of the pipeline can run end-to-end without
any real video on disk.

## What's here

- `README.md` - this file
- `goal.txt` - the user prompt
- `assets.json` - declared assets (so tests can build a Timeline without
  needing real media)
- `expected.plan.json` - rough expectation of what the planner should
  produce (used as a smoke check, not a byte-for-byte golden)
