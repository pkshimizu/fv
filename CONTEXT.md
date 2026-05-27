# fv

`fv` is a TUI file viewer/manager built on a Component Architecture: each on-screen area (Filer, Prompt, side panels) implements a `Component` trait and dispatches user intent through a global `Action` enum.

## Language

### Async File Operations

**Async Job**:
A long-running file operation — Copy, Move, Zip create, Zip extract, or Delete — executed on a background worker thread with progress reporting and cooperative cancellation.
_Avoid_: background task, async task, async operation.

**Phase**:
A labelled stage within an Async Job that determines what the user sees in the progress display. The defined phases are `Scanning`, `Copying`, `Moving`, `Zipping`, `Extracting`, `Deleting`, `Cancelling`.
_Avoid_: step, stage, state.

**Scan Phase**:
The opening phase of an Async Job that walks the source tree (or reads the zip archive header) once to determine the total file count. Cancellable. Total is unknown during this phase, so progress is rendered as `Scanning... N files`.

**Operation Phase**:
The main phase that follows Scan Phase — the actual `Copying` / `Moving` / `Zipping` / `Extracting` / `Deleting`. Total is fixed; processed count advances per file.

**Progress**:
A structured update sent from the worker to the Prompt as `{ phase, processed, total: Option<usize> }`. Rendered as e.g. `Copying 7/1234 files` or `Scanning... 23 files`.
_Avoid_: progress text, status message.

**File-level Checkpoint**:
The granularity at which an Async Job checks for cancellation and emits progress — between completed files, never mid-file. A 10 GB single-file copy cannot be interrupted until that file finishes.

**Cancel Token**:
An `Arc<AtomicBool>` shared between the Prompt and the worker. The Prompt sets it on Esc; the worker polls it at each File-level Checkpoint.
_Avoid_: cancellation signal, abort flag, kill switch.

**Filer Lock**:
The UI invariant that while an Async Job is in flight (including the `Cancelling` phase), only the Prompt receives keyboard input — Filer and side panels ignore keys. Implies `PromptMode::Progress` is treated as an active mode.
_Avoid_: modal block, input freeze.

**Partial Result**:
The set of fully-completed files an Async Job leaves behind when cancelled or aborted by error. Always retained on disk. **Exception**: the in-progress zip file produced by a cancelled Zip create is removed (matching the existing error-path behaviour of `create_zip`).
_Avoid_: leftover, residue, half-state.

### Example dialogue

> **Dev**: User hit Esc halfway through copying a directory. Do we delete what's already there?
>
> **Domain**: No, those are a **Partial Result** — keep them. The user can clean up if they want; rolling Move back the other way is a worse trade.
>
> **Dev**: What about Zip? Cancelling mid-zip leaves a `.zip` that won't open.
>
> **Domain**: Right, that's the one **Exception**. The unfinished archive isn't a useful Partial Result, so we delete it — same as the error path.
>
> **Dev**: And if the worker is mid-copy of a 10 GB file when Esc fires?
>
> **Domain**: The cancel only takes effect at the next **File-level Checkpoint**. So that file finishes, then the worker exits. We trade cancellation latency for "every file on disk is complete."
