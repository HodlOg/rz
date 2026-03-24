# rz — Known Issues

## Active

### Hub requires manual permission grant
The WASM plugin requests permissions on load but hides itself immediately.
Users must launch visibly first to approve, or use `load_plugins` in config
(which auto-grants on session start). Need a better first-run experience.

### Multiple hub instances cause duplicate messages
Using `--plugin file:...` in pipe commands launches new instances.
Fixed by piping via `--name rz` (broadcasts to running instances), but
if multiple instances exist, messages are delivered multiple times.
Mitigation: ensure only one instance via `load_plugins`.

### System rustc shadows rustup's for WASM builds
`/usr/bin/rustc` doesn't see rustup targets. Makefile uses explicit
`RUSTC=~/.rustup/toolchains/.../bin/rustc` override. Fragile.

### write_to_pane_id bypasses bracketed paste
Hub uses raw PTY write which doesn't trigger bracketed paste mode.
Works for @@RZ: envelopes but could cause issues with multi-line
payloads or special characters in message text.

### Hub installation friction
Users must build the WASM plugin from source (`cargo build --target wasm32-wasip1`)
or manually download the `.wasm` artifact and place it in the right path. No binary
distribution exists. This is the main barrier to adoption for non-Rust users.
Planned fix: GitHub releases with pre-built `.wasm` artifacts, and `rz init`
auto-downloads the matching hub version.

## Resolved
- ~~Hub routing blocks on pipe~~ → Fixed: pipe by name, not URL
- ~~Messages don't auto-submit~~ → Fixed: use \r (CR) instead of \n
- ~~cdylib WASM doesn't export _start~~ → Fixed: binary target
