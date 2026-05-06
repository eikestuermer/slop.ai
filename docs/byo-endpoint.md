# BYO LLM endpoint

Slop AI is local-first. We deliberately do not bundle a model or hardcode a
vendor. You bring your own OpenAI-compatible chat-completions endpoint, and
Slop AI talks to it.

This document covers:

- which local stacks we recommend and how to set them up,
- how to point Slop AI at a hosted endpoint (OpenAI, OpenRouter, etc.),
- privacy posture and the data we send,
- troubleshooting.

## Recommended local stacks

### Ollama + Qwen3 (default)

The simplest path. Ollama exposes an OpenAI-compatible endpoint at
`http://localhost:11434/v1` and supports structured outputs.

```bash
# install Ollama (https://ollama.com)
ollama pull qwen3:8b
ollama serve  # starts the server if not already running
```

In Slop AI, open the LLM endpoint config and set:

- Base URL: `http://localhost:11434/v1`
- Model: `qwen3:8b`
- API key: empty
- Temperature: `0`

Qwen3 is published under Apache-2.0, which makes it the cleanest license fit
for an open-source product. Smaller variants (`qwen3:4b`, `qwen3:1.7b`) work
on lower-end hardware with reduced plan quality.

### llama.cpp server

For maximum control over inference behavior:

```bash
# build llama.cpp with `make` or `cmake`
./server -m path/to/model.gguf -c 8192 --port 8080 --api-key any
```

Slop AI config:

- Base URL: `http://localhost:8080/v1`
- Model: any (llama.cpp ignores the model name on the OpenAI shim)

llama.cpp's OpenAI-compatible mode supports JSON Schema constraint via the
`response_format.json_schema` field, which is exactly what Slop AI sends.

### LM Studio

LM Studio's local server defaults to `http://localhost:1234/v1` and ships an
OpenAI-compatible API. Pick a model in the LM Studio UI, start the server,
and point Slop AI at it.

## Hosted endpoints

Slop AI does not call the cloud unless you tell it to. If you want to use a
hosted endpoint:

- **OpenAI**: Base URL `https://api.openai.com/v1`. Set the API key. Pick a
  model that supports structured outputs (`gpt-5.4-mini`, `gpt-5-mini`,
  `gpt-4o`, etc.).
- **OpenRouter**: Base URL `https://openrouter.ai/api/v1`. Same shape; pick
  any model their gateway supports.
- **Self-hosted vLLM**: Base URL `http://your-host:8000/v1`. vLLM accepts
  OpenAI-compatible requests including JSON Schema response formats.

When you point Slop AI at a non-localhost URL, the UI shows a yellow warning
explaining that the project goal and candidate metadata will be sent to that
endpoint. Media files never leave your machine; only metadata, transcripts,
and the project goal do.

## What we send to the endpoint

The planner request payload contains:

- the **goal** (your prompt),
- a list of **candidate moments** (`asset_id`, time ranges, transcript text,
  speaker tag, score, feature breakdown),
- a list of **assets** (id, duration, has_video, has_audio - no URIs),
- a list of allowed **track ids**,
- a JSON Schema describing the strict response shape.

We do not send:

- raw media files,
- file paths or URIs,
- proxy/preview MP4s,
- the project's op log.

If the endpoint is on `http://localhost`, `http://127.0.0.1`, or `http://[::1]`,
we treat it as local. Otherwise we treat it as cloud.

## Privacy mode

Slop AI ships a "privacy mode" toggle. Enabling it:

- forces the endpoint config to a localhost URL (rejects non-local URLs),
- disables all optional telemetry events,
- writes a `PRIVACY_MODE` file into the project root that future versions
  will use to gate any networked feature.

Privacy mode is the recommended default for sensitive material (interviews
under embargo, footage with NDA constraints, personal projects).

## Troubleshooting

### "endpoint returned 400" with structured outputs

Some local servers return 400 when they do not implement
`response_format.json_schema`. Slop AI requires this constraint. Recent
Ollama, llama.cpp server, and LM Studio versions all support it. Update to
the latest version.

### "model content was not valid JSON"

Some smaller models do not reliably produce valid JSON even when asked.
Workarounds:

- raise temperature to 0,
- switch to a 7B+ model (Qwen3 8B is a reliable lower bound),
- enable `--features whisper-cpp` so transcripts contain real text, which
  makes the candidate set richer and the model less likely to over-shorten.

### "plan validation failed after repair"

Either the model's response is structurally valid but referentially wrong
(unknown asset id, time outside the asset duration), or the repair pass
could not deterministically fix it. In the UI the planner status panel
surfaces both the validator error and the repair notes; that is the place to
start debugging. Pinning a few clips you know you want and re-running often
fixes it.
