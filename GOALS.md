# rz — Goals

## Vision
Minimal, CLI-native multi-agent communication over Zellij. Agents talk via pane IDs, not APIs.

## Current (v0.3)
- [x] CLI: send, broadcast, spawn, list, dump, log, status, ping, watch, close, rename, color
- [x] Protocol: @@RZ: JSON envelopes with threading (ref), message kinds (Chat, Ping, Pong, Hello, Error)
- [x] Workspace: shared dir for file-based collaboration (rz init/dir)
- [x] Blocking send: --wait with timeout
- [x] Long-lived agents: bootstrap tells agents to persist across tasks
- [x] Hub plugin: WASM plugin for native message routing via zellij pipe
- [x] Pane ID shorthand: "3" instead of "terminal_3"

## Next (v0.4)
- [ ] Heartbeat: agents signal responsiveness, hub detects stuck agents
- [ ] Auto-pong: hub replies to pings on behalf of registered agents
- [ ] Agent capabilities: register with tags, route by capability
- [ ] Dashboard: hub renders live agent status when visible
- [ ] `rz init` auto-loads hub plugin into current session

## Future
- [ ] External bridge: WebSocket sidecar for non-Zellij agents
- [ ] Message persistence: hub stores recent messages, queryable history
- [ ] Acknowledgements: delivery confirmation
- [ ] Rate limiting: prevent runaway agents from flooding
- [ ] tmux backend: abstract transport layer so rz works with tmux (send-keys/capture-pane) not just Zellij
- [ ] Raw socket transport: for non-terminal-multiplexer environments, direct TCP/Unix socket communication
- [ ] Pre-built hub binaries: GitHub releases with .wasm artifacts, rz init auto-downloads
- [ ] Package manager support: AUR, brew, nix

## Acknowledgments
rz is built on [Zellij](https://zellij.dev/) — its pane abstraction, WASM plugin system, and pipe IPC are the foundation that makes lightweight, zero-config agent communication possible.
