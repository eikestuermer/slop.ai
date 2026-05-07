//! Run the compiled filtergraph through `ffmpeg`.

use crate::compiler::{compile_timeline, CompiledRender, RenderOptions};
use slop_core::Timeline;
use std::path::Path;
use thiserror::Error;
use tokio::process::Command;

/// Errors during rendering.
#[derive(Debug, Error)]
pub enum RenderError {
    /// ffmpeg not found on PATH.
    #[error("ffmpeg not found on PATH")]
    BinaryNotFound,
    /// ffmpeg exited non-zero.
    #[error("ffmpeg exited {status}: {stderr}")]
    NonZeroExit {
        /// Exit code.
        status: i32,
        /// Captured stderr.
        stderr: String,
    },
    /// I/O.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Compile and render `tl` to `out`.
pub async fn render(
    tl: &Timeline,
    out: impl AsRef<Path>,
    opts: &RenderOptions,
) -> Result<CompiledRender, RenderError> {
    let out = out.as_ref().to_path_buf();
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }

    let compiled = compile_timeline(tl, opts);
    let cmd = build_ffmpeg_cmd(&compiled, &out, opts);
    let status = run(cmd).await?;
    if !status.success() {
        return Err(RenderError::NonZeroExit {
            status: status.code().unwrap_or(-1),
            stderr: String::new(),
        });
    }

    if let Some(srt) = &compiled.sidecar_srt {
        let srt_path = out.with_extension("srt");
        tokio::fs::write(&srt_path, srt).await?;
    }

    Ok(compiled)
}

fn build_ffmpeg_cmd(compiled: &CompiledRender, out: &Path, _opts: &RenderOptions) -> Vec<String> {
    let mut cmd: Vec<String> = vec!["-y".into()];
    for inp in &compiled.inputs {
        cmd.push("-i".into());
        cmd.push(strip_file_uri(inp));
    }
    cmd.push("-filter_complex".into());
    cmd.push(compiled.filtergraph.clone());
    cmd.push("-map".into());
    cmd.push(compiled.video_out.clone());
    if let Some(a) = &compiled.audio_out {
        cmd.push("-map".into());
        cmd.push(a.clone());
    }
    for arg in &compiled.encoder_args {
        cmd.push(arg.clone());
    }
    cmd.push(out.to_string_lossy().to_string());
    cmd
}

fn strip_file_uri(uri: &str) -> String {
    if let Some(rest) = uri.strip_prefix("file://") {
        rest.to_string()
    } else {
        uri.to_string()
    }
}

async fn run(args: Vec<String>) -> Result<std::process::ExitStatus, RenderError> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(&args);
    let status = cmd.status().await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            RenderError::BinaryNotFound
        } else {
            RenderError::Io(e)
        }
    })?;
    Ok(status)
}
