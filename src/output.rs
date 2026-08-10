//! Stdout/stderr write helpers shared by command handlers.
//!
//! Isolated so meta-commands (`doctor`, schema, …) do not depend on the full
//! `lib` dispatch graph (Rules: single responsibility / componentization).

use std::io::{self, Write};

use serde::Serialize;

use crate::error::{AppError, AppResult, ErrorDetail, InternalOp};

/// Map stdout I/O failures to domain errors (broken pipe → exit 141).
pub(crate) fn map_stdout_err(e: io::Error) -> AppError {
    if e.kind() == io::ErrorKind::BrokenPipe {
        AppError::broken_pipe()
    } else {
        AppError::of_with_source(
            ErrorDetail::Internal {
                op: InternalOp::StdoutWrite,
            },
            e,
        )
    }
}

/// Serialize one RFC 8259 JSON object (compact, single line) + trailing `\n`.
///
/// Product stdout is a single JSON document per invocation (not NDJSON streams).
/// Serialize to an intermediate buffer first so broken-pipe on write is mapped
/// without conflating pure serde failures with I/O.
pub(crate) fn write_json<Out: Write, T: Serialize>(stdout: &mut Out, v: &T) -> AppResult<()> {
    // Small default: version/doctor envelopes are a few hundred bytes; large
    // search payloads grow via try_reserve inside serde's writer on `buf`.
    let mut buf = Vec::with_capacity(256);
    serde_json::to_writer(&mut buf, v).map_err(|e| {
        AppError::of_with_source(
            ErrorDetail::Internal {
                op: InternalOp::JsonSerialize,
            },
            e,
        )
    })?;
    buf.push(b'\n');
    stdout.write_all(&buf).map_err(map_stdout_err)?;
    // Best-effort: payload already written; flush failure must not change exit code.
    let _ = stdout.flush();
    Ok(())
}
