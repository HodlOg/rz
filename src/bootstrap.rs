//! Bootstrap message sent to newly spawned agents.

use eyre::Result;

use crate::zellij;

/// Build bootstrap instructions for a newly spawned agent.
///
/// Includes: identity, how to communicate, who else is running.
pub fn build(pane_id: &str, name: Option<&str>, rz_path: &str) -> Result<String> {
    let panes = zellij::list_panes()?;
    let identity = name.unwrap_or(pane_id);

    let mut peers = String::new();
    for p in &panes {
        if p.is_plugin || p.pane_id() == pane_id {
            continue;
        }
        let cmd = p.pane_command.as_deref().unwrap_or("shell");
        let tab = p.tab_name.as_deref().unwrap_or("-");
        peers.push_str(&format!("  - {} ({}, tab: {})\n", p.pane_id(), cmd, tab));
    }
    if peers.is_empty() {
        peers.push_str("  (none)\n");
    }

    Ok(format!(
        r#"## Multi-Agent Environment

You are agent "{identity}" (pane: {pane_id}) in a multi-agent Zellij session.

### Communication

You have `rz` at `{rz_path}`. Use it to talk to other agents:

```bash
# Send a message to another agent
{rz_path} send <pane_id> "your message"

# Send raw text (no protocol envelope)
{rz_path} send --raw <pane_id> "raw text"

# List all agents
{rz_path} list

# Read another agent's scrollback
{rz_path} dump <pane_id>

# Stream another agent's output in real-time
{rz_path} watch <pane_id>

# Spawn a new agent
{rz_path} spawn <command>

# Broadcast to all agents
{rz_path} broadcast "message"
```

### Active agents

{peers}
### Protocol

When you receive a message starting with `@@RZ:` it is a protocol envelope.
The JSON inside has `from`, `kind`, and `ts` fields. Respond by using
`{rz_path} send <from_pane_id> "your response"`.

Plain text messages (no @@RZ: prefix) are direct human-style instructions."#
    ))
}
