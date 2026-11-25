# Security policy

## Supported versions

Security fixes are made for the latest release (or `main`, if no release has been tagged) only.

## Reporting a vulnerability

Do not open a public issue for security problems. Use GitHub's private vulnerability reporting
("Report a vulnerability" on the Security tab of github.com/DakodaStemen/lumen) if it is enabled;
otherwise contact the maintainer, [@DakodaStemen](https://github.com/DakodaStemen), directly.
Include the affected version or commit, a description, and steps to reproduce.

Lumen opens untrusted media files and network URLs through libmpv and runs Streamlink or yt-dlp helpers, so crashes caused by crafted media or playlists, command or argument injection through stream links, and leaks of link credentials into saved state are in scope. Problems inside libmpv or FFmpeg themselves should also be reported upstream.

## What not to commit

- Secrets: API keys, tokens, passwords, private keys, `.env` files.
- Personal data of any kind.

If you commit one by mistake, tell the maintainer straight away and rotate the credential first;
a new commit alone does not remove it from history.
