# 1. Async file operations: std::thread + AtomicBool, file-level checkpoint, locked Filer

Date: 2026-05-28
Status: accepted

## Context

Copy / Move / Zip create / Zip extract / Delete can take seconds to minutes on large trees, and today they run synchronously inside `execute_prompt_action`, freezing the TUI. We want them on a background thread with a progress display and a working cancel. Several plausible designs exist at each layer (threading primitive, cancellation granularity, partial-result policy, concurrency model), so a record of why we picked the conservative ones is worth keeping.

## Decision

- **One `std::thread` per Async Job**, no `tokio`, no pool. Worker holds `mpsc::Sender<ProgressMessage>`; `PromptComponent::tick()` drains the receiver. `ProgressMessage::Update` becomes structured: `{ phase: Phase, processed: usize, total: Option<usize> }` (replacing the current `Update(String)`).
- **Cancel via `Arc<AtomicBool>`**, polled at File-level Checkpoints (between files). Esc in `PromptMode::Progress` sets the flag and flips the displayed phase to `Cancelling`. On observing cancel the worker stops scheduling further files and exits with `ProgressMessage::Complete` (no dedicated `Cancelled` terminal variant — the silent close is enough).
- **Two-phase model** for operations whose total is not free: a `Scanning` phase walks the source tree first, emitting `Update { phase: Scanning, processed, total: None }`, then the operation phase starts with a fixed `total`. Zip extract skips Scan Phase since `archive.len()` is free.
- **One job at a time + Filer Lock.** A second start attempt while a job runs produces an error in the Prompt. `PromptMode::is_active()` returns `true` for `Progress` (currently `false`), so Filer and side panels do not receive keys while a job runs.
- **Partial Result retained on cancel and on error.** First error aborts the worker and surfaces `ProgressMessage::Error`. Exception: a cancelled or errored Zip create deletes its in-progress `.zip`, matching the existing error path in `create_zip`.

## Considered options

- **`tokio` runtime + `CancellationToken`**: rejected; adds a runtime dependency for a single-thread workload `std::thread` already covers.
- **Mid-file cancellation via a checking Reader/Writer wrapper**: rejected; conflicts with "completed files only" Partial Result and complicates `std::fs::copy` paths.
- **Rollback on cancel** (delete already-copied files, reverse Move): rejected; Move-rollback requires reverse copies that can themselves fail, multiplying failure modes.
- **Multiple concurrent jobs**: rejected; would require a jobs panel UI, per-job IDs, and per-job cancel keys — too much surface for current needs.
- **`mpsc` drop-as-cancel signal** (the pattern `execute_grep` uses with `child.kill()`): rejected; cooperative AtomicBool reads more clearly for in-process workers, and the grep pattern's value is the child-process kill — not unifiable cleanly.
- **Discover-and-process in parallel without Scan Phase**: rejected; numerator and denominator both moving reads as visually unstable.

## Consequences

- Cancellation latency is bounded by per-file work, not per-byte. A 10 GB single-file copy cannot be aborted until that file finishes; this is documented under `File-level Checkpoint` in CONTEXT.md.
- Move within one filesystem uses atomic `rename` — cancel feels instant per item. Cross-filesystem Move falls back to copy + remove and inherits the per-file cancel latency.
- The Filer cannot be navigated during a job. We considered allowing it but rejected: a moving cursor under an in-flight destructive op makes accidents too easy.
- The `#[allow(dead_code)]` markers on `ProgressMessage::Update`, `PromptComponent::start_progress`, and adjacent plumbing come off — code already shaped for this design lands.
