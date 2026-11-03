# Contributing

Preserve the reference app and its settings. Implement changes in this repository
and test them with a separate profile (`XDG_CONFIG_HOME` points at a scratch directory).

Run formatting, Clippy and the tests before submitting changes:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

The UI tests and the demo recorder (`scripts/record-demo.sh`, which writes a clip for your own
checking and is no longer part of the README) aim at widgets by name, and a test keeps those
names reporting their positions.

Changes to rendering, input or fullscreen also need a run in the real window, on
Wayland and X11 if you can. Report the checks you ran and their limitations; do not
claim feature parity or performance improvements from language choice alone.

Keep mpv calls in `engine.rs`, player behaviour in `session.rs`, queue ordering in
`queue.rs`, persistence in `store.rs`, key bindings in `keys.rs`, GL rendering in
`video.rs`, and layout and widgets in `src/ui/`. New behaviour belongs in `Session` with a test in
`tests/playback.rs`, not in the window code.

## Checks before you push

`make ci` runs `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`,
markdownlint, `cargo test --locked` and `cargo build --release --locked`. Run it before pushing.
Optional: `pre-commit install` runs fast checks on each commit (`.pre-commit-config.yaml`).

## Branches, commits and pull requests

- Branch prefixes: `feat/`, `fix/`, `docs/`, `chore/`, `ci/`, `test/`.
- PR titles follow Conventional Commits (`feat: ...`, `fix: ...`, `chore: ...`). One purpose per PR.
- Commit messages say what changed and why, in the imperative mood.
- Update `CHANGELOG.md` under Unreleased when behavior changes.
- Architecture decisions are recorded in [docs/adr](docs/adr/README.md).
