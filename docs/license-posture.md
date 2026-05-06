# License posture

Slop AI is shipped under MIT. Everything we statically link or distribute
must be compatible with that license.

## Direct dependencies

| Component | License | Notes |
| --- | --- | --- |
| Rust crates (serde, tokio, etc.) | MIT or Apache-2.0 | Standard |
| FFmpeg | LGPL-2.1+ (default) / GPL-2+ (with `--enable-gpl`) | We ship LGPL-only builds. Never enable `--enable-gpl`. |
| OpenTimelineIO | Apache-2.0 | If we link the C++ library at all, dynamic-linking only. |
| whisper.cpp | MIT | Default offline ASR. |
| OpenCV (optional, future) | Apache-2.0 (4.5+) | Optional feature for advanced face/motion detection. |
| OpenReelio shell (forked code) | MIT | Same license as ours; clean fork. |

## Local LLM models

Models are downloaded by the user, not bundled by Slop AI.

| Model family | License | Recommended |
| --- | --- | --- |
| Qwen3 | Apache-2.0 | Yes; the default we recommend in docs. |
| Mistral | Apache-2.0 | Yes; alternative recommended. |
| Llama 3.x | Llama Community License | Use only if you accept the terms. We document this; we do not recommend Llama as the default. |
| Gemma | Gemma Terms of Use | Use only if you accept the terms. We document this; we do not recommend Gemma as the default. |

When the user picks a model in the BYO endpoint UI, we do not validate the
license; we surface the license name we know about and link to the model's
upstream page.

## What "ship" means

We never bundle:

- model weights of any kind,
- proprietary codecs,
- GPL-only FFmpeg builds.

We do bundle:

- our own MIT-licensed source,
- LGPL-licensed FFmpeg static or dynamic libraries (LGPL-only),
- whisper.cpp source compiled at build time (MIT).

## CI checks

The CI pipeline runs `cargo deny check licenses` against an allow-list. New
dependencies that fall outside the allow-list block the build until reviewed
and either accepted (with a note) or removed.
