# rz

Agent-to-agent messaging over Zellij panes.

Spawn LLM agents (or any process) in Zellij panes and let them communicate directly — no files, no sockets, just pane IDs.

> **This is an experimental project.** The tool is built by the agents that use it. A human provides direction and ideas; agents design, implement, audit, and fix the code — coordinating through `rz` itself. Each session, the tool gets better at enabling the next session.

## Install

```bash
cargo install rz-cli
```

Requires [Zellij](https://zellij.dev) 0.42+.

**Hub plugin** (optional, enables name-based routing and timers):

```bash
# Option 1: Pre-built from GitHub releases
curl -L https://github.com/HodlOg/rz/releases/latest/download/rz-hub.wasm \
  -o ~/.config/zellij/plugins/rz-hub.wasm

# Option 2: Build from source
make install  # builds WASM and copies to ~/.config/zellij/plugins/
```

Then add to your Zellij config and set `RZ_HUB=1`.

## Quick start

```bash
# Initialize a session workspace
rz init

# Spawn an agent with a task
rz spawn --name researcher -p "find all TODOs in the codebase" claude

# Send a message (pane ID shorthand — just the number)
rz send 3 "summarize your findings"

# Block until the agent replies
rz send --wait 30 3 "what did you find?"

# See all agents
rz list
rz status

# View protocol messages only
rz log 3

# Read scrollback (last 50 lines)
rz dump 3 --last 50

# Broadcast to all agents
rz broadcast "wrap up your tasks"

# Set a timer — hub wakes you up
rz timer 60 "check build status"
```

## How it works

The core insight: LLMs already know how to use CLIs. Give them `rz send` and `rz spawn`, and multi-agent coordination emerges from conversation. No frameworks, no SDKs, no orchestration graphs — just a thin communication layer over Zellij panes.

### Standing on Zellij

`rz` is thin glue — [Zellij](https://zellij.dev) does the heavy lifting. Pane management, the WASM plugin system, `pipe` IPC, `write_to_pane_id` delivery, and `dump-screen` for reading output are all Zellij primitives. `rz` adds a messaging protocol, agent bootstrap, and a CLI that LLMs can use without documentation. Without Zellij's abstractions, this would be a much larger project.

### Architecture

```
┌──────────┐  rz send 3 "msg"  ┌──────────┐
│  Agent A │ ──────────────────>│  Agent B │
│ (pane 0) │                    │ (pane 3) │
└──────────┘                    └──────────┘
     │                               │
     │  zellij pipe --name rz        │
     ▼                               │
┌──────────┐  write_to_pane_id  ─────┘
│  rz-hub  │  (WASM plugin, optional)
│ (hidden) │
└──────────┘
```

**Direct mode** (default): `rz send` uses `zellij action paste` + CR to write messages to panes. Works everywhere, no setup. This is the only backend today.

**Hub mode** (`RZ_HUB=1`): Messages route through the `rz-hub` WASM plugin via `zellij pipe`. The hub maintains an agent registry, supports name-based routing, and delivers via `write_to_pane_id`. Add to your Zellij config:

> **Future:** Planned backends include tmux (for tmux users) and raw socket transport for non-terminal-multiplexer environments.

```kdl
load_plugins {
    "file:/path/to/rz-hub.wasm"
}
```

### Protocol

Messages use the `@@RZ:` wire format — a sentinel prefix + JSON:

```
@@RZ:{"id":"a1b2","from":"terminal_0","kind":{"kind":"chat","body":{"text":"hello"}},"ts":1774298000000}
```

Message kinds: `chat`, `ping`, `pong`, `hello`, `error`, `timer`.

Threading: reply with `--ref <message_id>` to link messages.

### Bootstrap

`rz spawn` automatically sends newly spawned agents:
- Their identity (pane ID, name)
- Full `rz` command reference
- List of active peers
- Workspace paths (if `rz init` was run)

Agents are told they are **long-lived** — they persist across tasks, accumulating context.

### Workspace

`rz init` creates a session workspace at `/tmp/rz-<session>/`:
- `shared/` — file drop for large outputs
- `goals.md` — session objectives (agents read on start)
- `context.md` — running log of decisions and discoveries
- `agents.md` — who's doing what

### Tickless timers

Agents can schedule wake-up calls through the hub:

```bash
rz timer 60 "check if deploy finished"
```

The hub uses `set_timeout()` — no polling. When the timer fires, the agent receives an `@@RZ:` Timer message. Useful for periodic monitoring, build checks, or goal reviews.

## Commands

| Command | Description |
|---------|-------------|
| `rz id` | Print this pane's ID |
| `rz init` | Initialize session workspace |
| `rz dir` | Print workspace path |
| `rz spawn` | Spawn an agent with bootstrap |
| `rz send` | Send a message (`--wait`, `--ref`, `--raw`) |
| `rz broadcast` | Message all agents |
| `rz list` | List panes with titles |
| `rz status` | Session overview with message counts |
| `rz log` | Show protocol messages from scrollback |
| `rz dump` | Read pane scrollback (`--last N`) |
| `rz watch` | Stream pane output in real-time |
| `rz ping` | Measure round-trip to an agent |
| `rz timer` | Schedule a wake-up (via hub) |
| `rz close` | Close a pane |
| `rz rename` | Rename a pane |
| `rz color` | Set pane color |

## The self-improving loop

Human designs the architecture and system — agents write the code. The human is not a product manager handing off feature requests; they're actively involved in the technical design, trade-offs, and abstractions. Agents handle implementation, testing, and auditing — coordinating through `rz` itself.

1. Human designs the system and key abstractions
2. Orchestrator agent decomposes into tasks, spawns specialists via `rz spawn`
3. Agents implement, test, and audit each other — coordinating via `rz send`
4. New version is installed mid-session; agents use what they just built

In a single session, 10+ agents collaborated to build the workspace restructure, WASM plugin, timer system, and code audit — all communicating through the tool they were building.

## License

MIT OR Apache-2.0
