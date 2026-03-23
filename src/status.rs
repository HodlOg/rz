//! Pane status summary for the `status` subcommand.

use crate::protocol::SENTINEL;
use crate::zellij::PaneInfo;

/// Per-pane status line.
pub struct PaneStatus {
    pub pane_id: String,
    pub title: String,
    pub command: String,
    pub running: bool,
    pub exit_status: Option<i32>,
    pub message_count: usize,
}

/// Summary of all panes.
pub struct StatusSummary {
    pub total: usize,
    pub running: usize,
    pub exited: usize,
    pub panes: Vec<PaneStatus>,
}

/// Count `@@RZ:` lines in a scrollback string.
fn count_messages(scrollback: &str) -> usize {
    scrollback.lines().filter(|l| l.contains(SENTINEL)).count()
}

/// Build a [`StatusSummary`] from a list of panes and a function that provides
/// each pane's scrollback.
///
/// The caller supplies `get_scrollback` so the function stays testable without
/// hitting real Zellij — in production, pass `|id| rz::zellij::dump(id)`.
pub fn summarize(
    panes: &[PaneInfo],
    get_scrollback: impl Fn(&str) -> Option<String>,
) -> StatusSummary {
    let mut running = 0usize;
    let mut exited = 0usize;
    let mut statuses = Vec::with_capacity(panes.len());

    for pane in panes {
        if pane.exited {
            exited += 1;
        } else {
            running += 1;
        }

        let pane_id = pane.pane_id();
        let msg_count = get_scrollback(&pane_id)
            .map(|s| count_messages(&s))
            .unwrap_or(0);

        let command = pane
            .pane_command
            .as_deref()
            .map(|c| {
                // basename only
                c.rsplit('/').next().unwrap_or(c)
            })
            .unwrap_or("-")
            .to_string();

        statuses.push(PaneStatus {
            pane_id,
            title: pane.title.clone(),
            command,
            running: !pane.exited,
            exit_status: pane.exit_status,
            message_count: msg_count,
        });
    }

    StatusSummary {
        total: panes.len(),
        running,
        exited,
        panes: statuses,
    }
}

/// Format the summary as a human-readable string.
pub fn format_summary(summary: &StatusSummary) -> String {
    let mut out = format!(
        "{} panes ({} running, {} exited)\n",
        summary.total, summary.running, summary.exited,
    );

    for p in &summary.panes {
        let state = if p.running {
            "running".to_string()
        } else {
            match p.exit_status {
                Some(code) => format!("exited ({})", code),
                None => "exited".to_string(),
            }
        };
        out.push_str(&format!(
            "  {} | {} | {} | {} | {} msgs\n",
            p.pane_id, p.title, p.command, state, p.message_count,
        ));
    }

    out
}
