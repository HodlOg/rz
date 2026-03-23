# rz

Agent-to-agent messaging over Zellij panes.

Spawn LLM agents (or any process) in Zellij panes and let them communicate directly — no files, no sockets, just pane IDs.

## Install

```bash
cargo install rz-cli
```

Requires [Zellij](https://zellij.dev) 0.44.0+.

## Usage

```bash
# Spawn an agent with bootstrap instructions + task prompt
rz spawn --name researcher -p "find all TODOs in the codebase" claude

# Send a message to an agent
rz send terminal_3 "summarize your findings"

# Send raw text (no protocol envelope)
rz send --raw terminal_3 "ls -la"

# List all running panes
rz list

# Read an agent's full scrollback
rz dump terminal_3

# Stream an agent's output in real-time
rz watch terminal_3

# Broadcast to all agents
rz broadcast "wrap up your tasks"

# Visual identification
rz color terminal_3 --bg "#003366"
rz rename terminal_3 researcher

# Close an agent
rz close terminal_3
```

## How it works

`rz` wraps Zellij's CLI:

| rz command | Zellij action |
|---|---|
| `rz send` | `paste --pane-id` + `write --pane-id 13` |
| `rz spawn` | `run -- <cmd>` (returns pane ID) |
| `rz list` | `list-panes --json` |
| `rz dump` | `dump-screen --pane-id --full` |
| `rz watch` | `subscribe --pane-id` |
| `rz close` | `close-pane --pane-id` |

## Protocol

Messages can be sent as plain text (`--raw`) or wrapped in an `@@RZ:` envelope:

```
@@RZ:{"id":"a1b20000","from":"terminal_0","kind":{"kind":"chat","body":{"text":"hello"}},"ts":1774298000000}
```

The `@@RZ:` prefix lets agents distinguish protocol messages from human input. The JSON is not base64-encoded — it's human-readable on the wire.

## Bootstrap

When you `rz spawn` an agent, it automatically receives:
- Its identity (pane ID and name)
- How to use `rz` (full command reference with the binary path)
- A list of all other active agents

This means spawned agents can immediately communicate without any manual setup.

## License

MIT OR Apache-2.0
