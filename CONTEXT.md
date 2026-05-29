# fv

`fv` is a TUI file viewer/manager built on a Component Architecture: each on-screen area (Filer, Prompt, side panels) implements a `Component` trait and dispatches user intent through a global `Action` enum.

## Language

### Selection

**Cursor File**:
The file currently highlighted in the Filer (returned by `FilerState::selected_file()`). Single-file operations such as Rename, Unzip, and Jump act on this one file.
_Avoid_: selected file, current file, focused file.

**Checked Paths**:
The set of paths the user has explicitly marked with the spacebar in the Filer (`FilerState::checked_paths`). Forms a multi-selection that persists across cursor movement.
_Avoid_: selected paths, marked files, tagged paths.

**Operation Targets**:
The actual files an action operates on, resolved by the rule "Checked Paths if non-empty, otherwise the Cursor File alone". Copy, Move, Delete, Zip create, and Yank all read targets through this rule. The resolved set **remembers its origin** — whether it came from the Cursor File or from Checked Paths — because downstream UX keys on it (e.g. Zip create's default archive name: the Cursor File's stem when single, a generic `files.zip` when multi-selected). Resolves to **nothing** when neither a Cursor File nor matching Checked Paths exist.
_Avoid_: targets, selection (both ambiguous between Cursor File and Checked Paths).

**Destination**:
The path the user supplies for a Copy or Move — where the Operation Targets go. Resolved by: when there is a **single** Operation Target and the path is **not** an existing directory, the path *is* the new top-level name (rename-on-copy/move — the result is created at exactly that path). Otherwise (multiple Operation Targets, or the path is an existing directory) the path is a **container directory** and each Operation Target is placed inside it under its own name; the container is created if missing. Resolution **never overwrites**: if a resolved path already exists, a `_1`, `_2`, … suffix is appended.
_Avoid_: target (ambiguous with Operation Targets), output path, folder.

**Yank**:
Read-only copy of the Operation Targets' absolute paths into the system clipboard, bound to `y`. Multiple paths are joined with `\n` and no trailing newline. Does not modify the filesystem or clear Checked Paths, so a `y` immediately followed by `c` / `m` can chain yank-then-copy/move on the same set.
_Avoid_: copy (overloaded with the Copy file action), clipboard write, pull to clipboard.

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

### Feedback

**Activity Indicator**:
An animated glyph shown next to an in-flight asynchronous operation — directory load, Grep, or an Async Job — to signal that the UI loop is still alive, i.e. the app is working rather than frozen. Distinct from **Progress**: Progress says *how much* is done (phase and counts); the Activity Indicator only says *that work is ongoing*. It is shown for indeterminate waits (directory load, Grep) and **alongside** Progress during an Async Job, where it keeps moving even when the processed count is momentarily stuck — e.g. copying a single large file with no intervening File-level Checkpoint.
_Avoid_: spinner (that names the glyph, not the concept), progress, loading flag.

### Example dialogues

#### On Selection

> **Dev**: User pressed `y` while three files are checked. What ends up in the clipboard?
>
> **Domain**: All three. Yank reads the **Operation Targets**, and Checked Paths win when they're non-empty. The Cursor File is ignored in that case.
>
> **Dev**: And the Checked Paths stay checked after yank?
>
> **Domain**: Yes. Yank is read-only, so we don't touch them. That way the user can yank, then immediately press `c` to copy the same set without re-checking.
>
> **Dev**: Empty Checked Paths, cursor on `foo.txt` — same key?
>
> **Domain**: Operation Targets falls through to just `foo.txt`. Single line in the clipboard, no trailing newline.

#### On Async File Operations

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
