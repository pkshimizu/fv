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

**Copy Plan**:
The flat work list a Copy or Move's Scan Phase produces and its Operation Phase executes: each entry is a concrete file copy or directory creation with its **Destination** already resolved (collision-avoided with `_1`, `_2`, … suffixes). A cross-filesystem Move builds the same Copy Plan for its copy-then-remove fallback. Decouples "decide what to do" (Scan) from "do it" (Operation), so the Operation Phase is a uniform per-entry loop regardless of source-tree shape.
_Avoid_: file list, queue, batch (the last is overloaded with progress-batching).

**Progress**:
A structured update sent from the worker to the Prompt as `{ phase, processed, total: Option<usize> }`. Rendered as e.g. `Copying 7/1234 files` or `Scanning... 23 files`.
_Avoid_: progress text, status message.

**File-level Checkpoint**:
The granularity at which an Async Job checks for cancellation and emits progress — between completed files, never mid-file. A 10 GB single-file copy cannot be interrupted until that file finishes.

**Cancel Token**:
An `Arc<AtomicBool>` shared between the Prompt and the worker. The Prompt sets it on Esc; the worker polls it at each File-level Checkpoint.
_Avoid_: cancellation signal, abort flag, kill switch.

**Filer Lock**:
The UI invariant that while a blocking asynchronous operation owns the Prompt — an **Async Job** (including the `Cancelling` phase) or a **Translation Request** — only the Prompt receives keyboard input; Filer and side panels ignore keys. Implies the owning `PromptMode` is treated as an active mode.
_Avoid_: modal block, input freeze.

**Partial Result**:
The set of fully-completed files an Async Job leaves behind when cancelled or aborted by error. Always retained on disk. **Exception**: the in-progress zip file produced by a cancelled Zip create is removed (matching the existing error-path behaviour of `create_zip`).
_Avoid_: leftover, residue, half-state.

### Translation

**Target Language**:
The human language the user wants file content translated **into** — a translation destination only, not the app's UI language (the UI stays English). Selected from a curated enum in Settings, defaulting to English. Maps to the **Translation Provider**'s target-language parameter. The **source** language is never set by the user — the Provider auto-detects it.
_Avoid_: display language, UI language, locale (all imply the app's own UI language, which this is not), source language.

**Translation Provider**:
The swappable backend that turns text in an unknown source language into the **Target Language**. DeepL is the first (and currently only) Provider; the abstraction exists so others can be added later. Requires a user-supplied credential (the DeepL auth key, stored in settings). Distinct from an **Async Job**: a Provider does network work, not filesystem work.
_Avoid_: translation API, translation service, translator (reserve for the abstraction, not a concrete vendor).

**Translation Request**:
A single cancellable, asynchronous round-trip to the **Translation Provider** that translates the text currently loaded in the Preview side panel into the **Target Language**. Not an **Async Job** (which is strictly a file operation): it holds the **Filer Lock** while in flight, but it does **not** use the Async Job's **Cancel Token** — a Request has no **File-level Checkpoint** to poll, so Esc cancels by *abandoning the wait* (dropping the result receiver, releasing the Filer Lock immediately; the in-flight HTTP request finishes in the background and its result is discarded, the same receiver-drop cancellation the file-info load uses). The provider's quota is still consumed. Unlike an Async Job it reports no `processed/total` **Progress** — a Request is a single indeterminate wait, so the user sees only an **Activity Indicator** with the label `Translating...`.
_Avoid_: translate job, async translation (job implies the file-operation Async Job).

### Feedback

**Activity Indicator**:
An animated glyph shown next to an in-flight asynchronous operation — directory load, Grep, file info load, or an Async Job — to signal that the UI loop is still alive, i.e. the app is working rather than frozen. Distinct from **Progress**: Progress says *how much* is done (phase and counts); the Activity Indicator only says *that work is ongoing*. It is shown for indeterminate waits (directory load, Grep, file info load) and **alongside** Progress during an Async Job, where it keeps moving even when the processed count is momentarily stuck — e.g. copying a single large file with no intervening File-level Checkpoint.
_Avoid_: spinner (that names the glyph, not the concept), progress, loading flag.

**System Info**:
The live host-environment readout shown in the header — OS, kernel, hostname (the unchanging facts, in the header box title) plus CPU load, memory use, and uptime (the changing figures, refreshed about once a second). It is ambient, always-on context about the machine fv runs on, not about any file or operation. Distinct from file metadata (which describes the **Cursor File**) and from **Progress** (which describes an in-flight Async Job).
_Avoid_: system status, machine info, stats bar.

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
