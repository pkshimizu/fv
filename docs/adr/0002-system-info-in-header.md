# 2. System info: header-resident, synchronous ~1s refresh on the main tick

Date: 2026-05-31
Status: accepted

## Context

We want to surface host environment stats (OS, kernel, hostname, CPU%, memory, uptime) inside fv so the user can glance at system state during file work (issue #203). Two shapes were plausible at several layers: where the info lives (a side panel vs the always-visible header), how it is gathered (a background worker like file-info/directory-load vs the main thread), and how often it updates (one-shot snapshot vs live). The existing async machinery (`spawn_async_job`, `spawn_file_info`, the directory-load thread) all push heavy work onto worker threads, so refreshing system info on the main thread is a deliberate departure worth recording.

## Decision

- **Header-resident, not a side panel.** System info renders in the header box (`ui/features/header.rs`): static fields (OS, kernel, hostname) in the box **title**, dynamic fields (CPU, memory, uptime) in the content row's **left zone**, with the right zone reserved for a future clock. No keybinding, no `SidePanel` variant. The header is always visible, which matches "keep system state in view" better than an openable panel.
- **Synchronous refresh on the main tick.** `os::system_info::SystemInfoReader` holds a reused `sysinfo::System` and refreshes on `AppContext::tick()`. It does **not** use a worker thread. The system is created with `System::new()` (empty — no process enumeration) and only `refresh_cpu_usage()` + `refresh_memory()` are called, keeping each refresh sub-millisecond, so blocking the main loop is acceptable.
- **~1s throttle, internal to the reader.** A tick counter (`RefreshThrottle`) refreshes the dynamic fields every 4th tick (tick ≈ 250 ms → ≈ 1 s). Static fields are gathered once in `new()` and never re-read. CPU% accuracy needs ≥ ~200 ms between samples, which 1 s comfortably satisfies.
- **sysinfo hidden behind `os::system_info`.** The crate is confined to that module; the header and `AppContext` see only the `SystemInfo` value type and the reader, so swapping sysinfo later is localized.

## Considered options

- **Worker thread (mirroring file-info / directory-load):** rejected. Those use workers because their work is heavy (full-file reads, directory walks). A targeted sysinfo refresh is cheap enough for the main thread; a worker would add channel/handle plumbing for no latency benefit and complicate the always-on refresh cadence.
- **One-shot snapshot at open / async one-shot:** rejected. Memory, CPU, and uptime are dynamic; the user asked for a live header reading.
- **Side panel (like Attribute / File Info):** rejected. A panel needs opening and steals screen space; the value here is an at-a-glance, always-visible reading.
- **Refresh every tick (~250 ms):** rejected. No reading value over ~1 s for a human, and it churns the render loop needlessly.

## Consequences

- The first frame shows `CPU 0%` until the first ~1 s refresh produces a real sample (CPU% needs two spaced reads). Acceptable.
- Future header additions (storage info, clock) follow this same shape: a source contributing a segment, refreshed on the main tick, composed in `render_header`. The left-zone/right-zone split is already in place for them.
- If a future stat is genuinely expensive to gather (unlike CPU/memory), this decision would need revisiting for that stat specifically — it does not commit *all* future header data to main-thread gathering, only the cheap host stats.
