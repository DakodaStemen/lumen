# Packaging

| OS | Artifacts | libmpv |
| --- | --- | --- |
| Windows | `.zip` (portable), `.msi` (WiX via cargo-packager) | `libmpv-2.dll` from shinchiro release `20261021`, exact asset `mpv-dev-x86_64-20261021-git-e471496c56.7z`; **GPL-enabled**, no LGPL variant listed in that release |
| macOS | `.dmg` | Homebrew `mpv` (floating; reviewed formula: 0.41.0 revision 12, GPL-2.0-or-later AND LGPL-2.1-or-later), plus dependencies copied to `Contents/Frameworks` |
| Linux | `.deb`, `.rpm`, AppImage, Flatpak | deb/rpm use distro libmpv; AppImage uses Ubuntu runner packages (floating, exact closure/license unverified); Flatpak builds mpv v0.40.0 + FFmpeg n7.1.1 with **GPL enabled**, including FFmpeg version3 |

## Bundled-library licensing and source/relinking

Lumen's source is MIT; bundled binaries are **not necessarily MIT-only or
LGPL-only**. Dynamic linking does not remove GPL obligations. The Windows
archive is verified before extraction with SHA-256
`efc719eff3dc5954b4a5efc800632808bcec01cc9034940dd14e5eb7fe9ea633`
(GitHub release asset 660943567 API digest, also independently hashed after
`gh release download`). Change the tag, exact asset name and digest together.

[THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md#bundled-libmpv-inventory-and-source)
records platform-specific provenance, licenses, source locations, replacement
and rebuild instructions, and the unresolved corresponding-source inventory.
Users can replace the DLL next to the Windows executable, dylibs in
`Contents/Frameworks`, libraries in an extracted AppImage, or rebuild Flatpak's
source modules; replacements must preserve ABI compatibility. Rebuild Lumen
from the matching release commit and locked crates to relink the LGPL Rust
bindings. Distro package sources come from matching `apt-get source mpv` /
distro source repositories; Homebrew source URLs/patches come from the matching
formula revision; Flatpak source URLs/tags/options are in the manifest.

**Before publishing:** retain matching source, patches, build scripts and all
license/copyright notices for the complete bundled dependency graph, review
GPL combined-distribution requirements, and provide the necessary relinking
materials for LGPL components. Upstream links alone are not a complete source
offer. The workflows do not currently capture all these materials; this is an
explicit release-maintainer gate, not a claim that existing bundles comply.

Icons: masters in `assets/brand/`, generated sets in `packaging/icons/` (see `docs/brand.md`). `scripts/install.sh` does a per-user Linux install of the binary, all hicolor sizes and the launcher. The app id is `lumen-rs` natively (`.desktop` id, `StartupWMClass`, Wayland app_id) and the Flatpak id inside the Flatpak (read from `FLATPAK_ID`).

Everything is built by `.github/workflows/release.yml` (manual run, or a `v*` tag). The Linux job runs the
tests first, and a final job publishes `SHA256SUMS` for every artifact. Nothing is attached to a
GitHub Release automatically; that stays a manual step. Local builds:

```sh
cargo install cargo-packager --locked
cargo packager --release --formats deb        # or appimage / app / wix
cargo install cargo-generate-rpm && cargo build --release && cargo generate-rpm
flatpak-builder --user --install --force-clean build packaging/flatpak/io.github.DakodaStemen.Lumen.yml
```

## Not done / needs credentials

* **Windows signing**: needs an Authenticode certificate (OV/EV or Azure Trusted Signing). Installers are unsigned, so SmartScreen will warn.
* **macOS signing and notarization**: needs an Apple Developer ID certificate and notarytool credentials. Unsigned dmgs need right-click, Open. Separate arm64 and x86_64 dmgs are built; there is no universal binary.
* **Flatpak**: the manifest builds libass, FFmpeg, libplacebo, libXpresent and mpv from source, with git sources pinned to commits, and vendors the crates through `cargo-sources.json` (CI fails when that file falls behind `Cargo.lock`). It has not been submitted to Flathub. Streamlink and yt-dlp are not included, so Twitch and YouTube page links only play directly inside the Flatpak.
* **MPRIS / media keys**: implemented through `souvlaki` (`src/media_keys.rs`). The Flatpak owns `org.mpris.MediaPlayer2.lumen_rs`. Windows SMTC and macOS Now Playing go through the same crate but are untested.
* **macOS "open with"**: Finder delivers files through Apple events, not argv, so opening a file from Finder starts Lumen without loading it. Drag-and-drop onto the window works.
* The streams integration tests use POSIX shell scripts as fake tools and run on Unix only.
