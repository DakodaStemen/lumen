# ADR-0001: Rust with egui/eframe and libmpv for playback

| Field | Value |
| :---- | :---- |
| Status | Accepted |
| Date | 2026-11-03 |
| Deciders | @DakodaStemen |

## Context

Lumen is a media player. The code shows the choices already made: Rust (edition 2024, `rust-version` 1.88 in `Cargo.toml`), egui/eframe for the interface, libmpv through the `libmpv2` crate for playback, `souvlaki` for media keys and MPRIS, and Streamlink or yt-dlp helpers for stream URLs.

## Decision

We will build Lumen as a single Rust binary that renders the interface with egui/eframe and delegates decoding and playback to libmpv. mpv calls stay in `src/engine.rs`, player behavior in `src/session.rs`, and queue ordering in `src/queue.rs` (see CONTRIBUTING.md).

## Alternatives considered

| Alternative | Why not chosen |
| :---- | :---- |
| Qt or libVLC interface | Not used; the project is a Rust binary with no Qt, Python or libVLC runtime. |
| Implementing decoding in Rust | Not used; libmpv already handles formats, hardware decoding and subtitles. |

## Consequences

**Positive**

- libmpv supplies broad format support; the interface and session logic stay in Rust.
- Playback and stream behavior is covered by headless tests that drive a real libmpv.

**Negative**

- libmpv must be installed at build and run time (CI installs `libmpv-dev`; Windows needs a bundled library).
- Behavior depends on the installed mpv version; CI runs a job against a newer libmpv.
- Rendering and input need checks in a real window in addition to the tests.

**Follow-ups**

- None recorded.
