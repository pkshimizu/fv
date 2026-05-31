# 2. Preview translation: user-supplied DeepL key behind a swappable Provider, modelled as a cancellable async that shares the Filer Lock but not the Cancel Token

Date: 2026-05-31
Status: accepted

## Context

The Preview side panel shows the text of the **Cursor File**. We want a key (`t`) that translates that text into a user-chosen **Target Language** and toggles back to the original. The text comes from an external machine-translation service, which raises three decisions a future reader would otherwise question: which service (and whether it needs a credential), where translation sits relative to the existing **Async Job** machinery, and how it cancels. We surveyed keyless options before settling, so the rejection is worth recording.

## Decision

- **A `Translation Provider` abstraction, DeepL as the first and only implementation.** Lives in a new top-level `src/translate/` module (`mod.rs` trait, `deepl.rs`, `task.rs`) — translation is network work, not filesystem (`fs/`) or persistence (`store/`) work. The trait exists so the vendor can be swapped later without touching the Preview panel.
- **User-supplied DeepL auth key, stored in `settings.json`** (`store/settings.rs` `Settings`), entered in-app from the Settings panel via `PromptMode::Text`. The source language is never configured — DeepL auto-detects it; only the **Target Language** (a curated `serde` enum like `StartupDirectory`, default English) is user-selectable.
- **Translation is a `Translation Request`, a distinct concept from an Async Job.** It reuses the **Filer Lock** (generalised in CONTEXT.md from "while an Async Job is in flight" to "while a blocking async operation owns the Prompt") but reports no `processed/total` **Progress** — a single round-trip is one indeterminate wait, shown as an **Activity Indicator** labelled `Translating...`.
- **Cancel by abandoning the wait, not by the Cancel Token.** A Request has no **File-level Checkpoint** to poll, so Esc drops the result receiver and releases the Filer Lock immediately; the in-flight HTTP request finishes in the background and its result is discarded (the same receiver-drop cancellation the file-info load uses). Provider quota is still consumed.
- **Whole loaded preview translated, cached by Target Language.** The entire text already loaded in Preview (bounded by Preview's own 10,000-line / 100 MB limits) is sent as a line array so line count is preserved. The result is cached against the Target Language it was produced for; toggling reuses the cache on a match and issues a fresh Request on a mismatch.
- **Failures surface via `PromptMode::Error`.** Missing key, invalid key (401), quota exceeded (456), and network errors all release the Filer Lock and leave Preview on the original text. Missing key points the user to Settings.
- **A synchronous HTTP client (`ureq`), no `tokio`** — matches the `std::thread` + `mpsc` model the codebase already uses for `file_info` and Async Jobs.

## Considered options

- **Keyless services (MyMemory, Lingva, self-hosted LibreTranslate)**: rejected. MyMemory needs no key but caps at 500 bytes/request and ~5,000 chars/day anonymously with weaker quality; Lingva is an unofficial Google-Translate proxy dependent on third-party public instances (uptime + ToS risk); LibreTranslate is keyless only when self-hosted, which defeats "no pre-setup". DeepL API Developer issues a free key (1M chars total, no credit card) by copying it from an account page — low enough friction to be worth the one-time setup for materially better quality.
- **API key in an environment variable only**: rejected for the default path; it keeps the secret off disk but breaks the "configure entirely in-app" goal. Could be added later as an override.
- **Extending Async Job to "any long-running operation" with a `Translating` phase**: rejected; it would mix network and filesystem operations under one vocabulary and force a `processed/total` model that a single request can't honestly produce.
- **Translating only the visible viewport, re-translating on scroll**: rejected; minimal per-request size but repeated latency and quota cost on every scroll, plus fiddly caching.
- **True mid-request cancellation** (abortable HTTP client): rejected; needs an async runtime or a cancellation-aware client for marginal benefit, since a single translation round-trip is short and the receiver-drop pattern is already established.

## Consequences

- Translation does not work out of the box — the user must obtain and enter a DeepL key first. This is the deliberate price of translation quality over zero-setup.
- The DeepL auth key sits in plaintext in `settings.json` under the config directory (same posture as tokens stored by tools like `gh`).
- Esc during translation feels instant, but the request still completes server-side and consumes quota — documented under `Translation Request` in CONTEXT.md.
- The Settings panel grows from a single left/right selector to multiple rows (Startup Directory, Target Language, DeepL API Key), requiring row navigation.
- `Filer Lock` now spans two distinct operation kinds; its CONTEXT.md definition was generalised accordingly.
