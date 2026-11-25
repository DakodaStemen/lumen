# Changelog

## Unreleased

- New app icon: a rounded play triangle with a cursor pill on a green tile, with a simpler master for
  16 to 32 px. It is used for the window icon, the Linux hicolor theme (16 to 512 px plus scalable SVG,
  in the `.desktop` launcher, deb, rpm and Flatpak), the Windows `.ico` and the macOS `.icns`.
  Masters are in `assets/brand/`, generated files in `packaging/icons/` (`scripts/gen-icons.py`).
  `scripts/install.sh` installs the binary, icons and launcher for the current user.
- Fixes from an audit of the whole repository:
  - Queue: removing a row from its context menu no longer leaves the selection on the wrong row.
    A file that will not open is skipped for the next one (it stops when every entry has failed,
    even with repeat on). Playlists starting with a byte-order mark no longer gain a bogus first
    entry, and a downloaded HLS `.m3u8` is queued as one item instead of one per segment.
  - Profile: saves run on a background thread and collapse during a volume drag. Two Lumen
    windows no longer erase each other's resume points and bookmarks (entries are merged, under a
    file lock on Unix). A bad entry in `files.json` or a bad value in `settings.json` drops only
    that entry or value, and a file that cannot be read at all is kept as `*.broken`.
  - Streams: a helper started just as its job was cancelled is now stopped. Helpers end with Lumen
    on Linux even after a crash. Streamlink runs with `--no-config` when it supports it. Secrets in
    `srt://`, `ftp://`, `smb://` and other links are left out of the recent list.
  - Interface: the window redraws once a second when nothing is playing, instead of five times.
    Only visible queue rows are laid out. The seek and volume bars have names and values for
    screen readers and work with the arrow keys, Page Up/Down, Home and End. Menu check marks,
    the playing and selected queue rows and on-screen messages reach screen readers. Dimmed text
    and text on the accent colour meet WCAG AA (4.5:1) in both themes and with any desktop accent.
  - Keyboard: switching Preferences tabs cancels a waiting key capture. The link dialog no longer
    holds focus on every frame, so Tab reaches its buttons. The Enter that opens the dialog no
    longer submits it. A focused button or row keeps Space and Enter, but other shortcuts still
    work. Delete does nothing while a dialog is open. `Ctrl++` can be bound.
  - Files whose names are not valid UTF-8 are reported instead of failing to open silently.
  - Multiple shaders work on Windows. Absurd SponsorBlock times can no longer overflow.
  - Packaging: the Flatpak's crate list was out of date (39 crates missing); it was regenerated,
    and CI now checks it. The Flatpak's git sources are pinned to commits, its window matches its
    desktop file on Wayland, and MPRIS works in the sandbox. The release workflow tests before
    packaging, pins every action to a commit and publishes SHA-256 sums. The metainfo release date
    is corrected.

- A minimal look that follows the desktop: light or dark and the accent colour from the
  freedesktop portal (with `gsettings` as a fallback, and live changes), the system interface and
  monospace fonts through fontconfig, and libadwaita-like colours. Flat buttons, no gradients or
  branding on the welcome page, a toolbar with Open… and Open link…, the video edge to edge, and the
  status line replaced by a message over the picture only when there is something to say. The mini
  player button moved to the Video menu (Ctrl+M); the volume percentage is the volume bar's tooltip.
- The network dialog starts with `https://` filled in. Links typed without a scheme
  (`twitch.tv/name`, `www.youtube.com/watch?v=…`, `//host/path`) get `https://`; a whole link
  pasted after the prefix replaces it instead of producing `https://https://…`; schemes are
  lower-cased and surrounding whitespace dropped. Local paths are never turned into links.
- Quality defaults: mpv `profile=gpu-hq` for the window, a hardware-decoding picker and an HDR
  tone-mapping choice; the active decoder appears in Media information. HDR is shown tone-mapped to SDR
  (the video framebuffer is 8-bit RGBA).
- Subtitle font, size, colours, outline and position; brightness, contrast, saturation, gamma and hue;
  aspect, rotate and crop in the Video menu.
- Night mode (loudness normalisation), ReplayGain and an audio output device picker.
- A Shaders menu listing files from `~/.config/lumen-rs/shaders` (none are bundled).
- Media keys and MPRIS through a worker thread.
- A `.desktop` file, AppStream metainfo and a draft Flatpak manifest.
- README rewritten without the demo GIF (removed from the repository).
- Visual refresh and documentation polish: architecture diagram, documentation nav links and badge formatting.

## 0.3.0 — A friendlier interface

- Lumen's layout: sidebar with Open media, Network stream and a filterable queue; "Now playing"
  header with Audio & subtitles, Video and … menus; framed video; transport row.
- Welcome screen with recent files, drag-and-drop hint, spinners, and messages shown over the picture.
- Seek bar with hover time and chapter name, chapter ticks and A-B loop band; click to pause,
  double-click for fullscreen, scroll for volume, right-click for the menus.
- Native file pickers, Save queue (M3U), Clear, queue context menu.
- Preferences, with a keyboard tab that rebinds any key and refuses conflicts.
- Mini player, always on top (not on Wayland), media information, chapter, bookmark and recent menus.
- Load subtitle files, which are remembered for the file.
- Escape closes the top-most dialog before it acts as a shortcut.
- A hint when yt-dlp and Streamlink are missing for a site link.
- Interface tests that click and type through the real interface without a window.
- A demo GIF recorded from the real player, and the recorder (`scripts/record-demo.sh`).
- The welcome text no longer shows through the drop hint while files hover over the window.

## 0.2.0 — Streams

- Live streams through Streamlink, run as a local relay, with the stream title, a quality
  picker, low-latency mode and a "live" display with no resume position.
- YouTube and other sites through yt-dlp, with separate audio streams attached, and a
  fall-back to the link itself when yt-dlp cannot read it.
- SponsorBlock segment skipping using the privacy-preserving hash-prefix lookup.
- "Open network stream" dialog (Ctrl+L).
- Lookups run on worker threads and are cancelled or ignored when the user plays something else.

## 0.1.0 — Native preview

- Native Rust desktop app using egui/eframe and libmpv, drawing video on Wayland and X11.
- Playback core: queue with shuffle, repeat, M3U import and folder continuation; resume;
  per-file audio and subtitle choices, sync offsets and bookmarks.
- Seek bar, frame stepping, chapters, A-B loop, speed, volume to 200%, track and sync controls.
- Rebindable keys, fullscreen, screenshots.
- Independent profile in `~/.config/lumen-rs`.

This is an early migration preview. See `docs/migration.md` for remaining parity work.
