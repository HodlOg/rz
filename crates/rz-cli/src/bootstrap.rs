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

    // Check if workspace exists.
    let workspace = std::env::var("ZELLIJ_SESSION_NAME")
        .ok()
        .map(|s| format!("/tmp/rz-{s}"))
        .filter(|p| std::path::Path::new(p).exists());

    let workspace_section = if let Some(ref ws) = workspace {
        format!(
            r#"### Workspace

A shared workspace is available at `{ws}/shared/`.
Write large outputs (research, code drafts, logs) there instead of
inlining them in messages. Reference the file path in your message, e.g.:
`{rz_path} send 0 "findings at {ws}/shared/research.md"`

### Project Files

The workspace has coordination files that all agents should use:

- **`{ws}/goals.md`** — READ this when you start. It describes session goals. Add sub-goals as you discover them.
- **`{ws}/context.md`** — UPDATE this with important decisions, discoveries, and context as you work.
- **`{ws}/agents.md`** — UPDATE this with your pane ID, name, and current task when you start or switch tasks.
"#
        )
    } else {
        String::new()
    };

    Ok(format!(
        r#"## Multi-Agent Environment

You are agent "{identity}" (pane: {pane_id}) in a multi-agent Zellij session.

You are **long-lived** — you will receive multiple tasks over time, not just one.
After completing a task, report back and wait for the next one. Your context
and knowledge accumulate across tasks, making you more valuable over time.
Do not exit after finishing a task.

### Communication

You have `rz` at `{rz_path}`. Use it to talk to other agents:

```bash
# Send a message to another agent (use just the number)
{rz_path} send <pane_id> "your message"

# Send and block until reply (timeout in seconds)
{rz_path} send --wait 30 <pane_id> "question"

# List all agents
{rz_path} list

# Session overview with message counts
{rz_path} status

# Read another agent's scrollback (last N lines)
{rz_path} dump <pane_id> --last 50

# View protocol messages only
{rz_path} log <pane_id>

# Broadcast to all agents
{rz_path} broadcast "message"
```

{workspace_section}### Active agents

{peers}
### Protocol

When you receive a message starting with `@@RZ:` it is a protocol envelope.
The JSON inside has `from`, `kind`, and `ts` fields. Reply with
`{rz_path} send --ref <message_id> <from_pane_id> "your response"`.

Keep messages short. Use the workspace for large outputs."#
    ))
}
