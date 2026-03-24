//! `rz` — inter-agent messaging over Zellij.

use clap::{Parser, Subcommand};
use eyre::{Result, bail};

use rz_cli::protocol::{Envelope, MessageKind};
use rz_cli::{bootstrap, log, status, zellij};

/// Agent-to-agent messaging over Zellij panes.
///
/// Uses Zellij's CLI for direct, targeted communication between processes
/// running in Zellij panes. No files, no sockets — just pane IDs.
///
/// Quick start:
///   rz spawn claude                     # start an agent, get its pane ID
///   rz send terminal_3 "do something"   # send it a message
///   rz list                             # see all running panes
///   rz dump terminal_3                  # read what it's been doing
///   rz watch terminal_3                 # stream its output in real-time
///   rz broadcast "status update"        # message all agents
#[derive(Parser)]
#[command(name = "rz", version, about, long_about)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print this pane's ID.
    Id,

    /// Initialize a shared workspace for this session.
    ///
    /// Creates a directory at /tmp/rz-<session>/ with a shared/ folder
    /// and prints the path. Agents can write files there instead of
    /// sending large messages. Idempotent — safe to call multiple times.
    Init,

    /// Print the session workspace path.
    ///
    /// Fails if `rz init` hasn't been run yet.
    Dir,

    /// Spawn an agent in a new pane with communication instructions.
    ///
    /// Creates a new Zellij pane, waits for it to start, then sends
    /// bootstrap instructions (identity, rz usage, active peers).
    ///
    /// Examples:
    ///   rz spawn claude
    ///   rz spawn --name researcher -p "find all TODOs" claude
    ///   rz spawn --no-bootstrap python agent.py
    Spawn {
        /// Command to run.
        command: String,
        /// Pane name (visible in Zellij frame).
        #[arg(short, long)]
        name: Option<String>,
        /// Skip bootstrap instructions.
        #[arg(long)]
        no_bootstrap: bool,
        /// Seconds to wait for process to start before bootstrapping.
        #[arg(long, default_value = "8")]
        wait: u64,
        /// Task prompt to send after bootstrap.
        #[arg(short, long)]
        prompt: Option<String>,
        /// Color the pane background for visual identification (hex, e.g. "#003366").
        #[arg(long)]
        color: Option<String>,
        /// Extra arguments passed to the command.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Send a message to a pane.
    ///
    /// By default wraps the message in an @@RZ: protocol envelope with
    /// sender ID and timestamp. Use --raw for plain text.
    ///
    /// Examples:
    ///   rz send 3 "research this topic"
    ///   rz send --raw terminal_3 "ls -la"
    ///   rz send --ref abc123 terminal_3 "replying to your message"
    ///   rz send --wait 30 3 "do this and reply"
    Send {
        /// Target pane ID (e.g. terminal_1, or just 1).
        pane: String,
        /// Message text.
        message: String,
        /// Send plain text instead of @@RZ: envelope.
        #[arg(long)]
        raw: bool,
        /// Sender identity. Defaults to ZELLIJ_PANE_ID.
        #[arg(long)]
        from: Option<String>,
        /// Reference a previous message ID (for threading).
        #[arg(long)]
        r#ref: Option<String>,
        /// Block until a reply (with matching ref) arrives in own scrollback.
        /// Value is timeout in seconds.
        #[arg(long)]
        wait: Option<u64>,
    },

    /// Broadcast a message to all other terminal panes.
    Broadcast {
        /// Message text.
        message: String,
        /// Send plain text instead of @@RZ: envelopes.
        #[arg(long)]
        raw: bool,
    },

    /// List all panes with their commands and status.
    List,

    /// Show a summary of the session: pane counts and per-pane status.
    ///
    /// Includes message counts from each pane's scrollback.
    Status,

    /// Dump a pane's scrollback to stdout.
    ///
    /// Examples:
    ///   rz dump 3              # full scrollback
    ///   rz dump 3 --last 50    # last 50 lines only
    Dump {
        /// Target pane ID.
        pane: String,
        /// Only show the last N lines.
        #[arg(long)]
        last: Option<usize>,
    },

    /// Show @@RZ: protocol messages from a pane's scrollback.
    ///
    /// Extracts and formats all protocol envelopes, filtering out
    /// normal shell output.
    ///
    /// Examples:
    ///   rz log 3
    ///   rz log terminal_1 --last 10
    Log {
        /// Target pane ID.
        pane: String,
        /// Only show the last N messages.
        #[arg(long)]
        last: Option<usize>,
    },

    /// Stream a pane's output in real-time (uses zellij subscribe).
    ///
    /// Prints viewport changes as they happen. Press Ctrl+C to stop.
    Watch {
        /// Target pane ID.
        pane: String,
        /// Output as JSON (NDJSON).
        #[arg(long)]
        json: bool,
    },

    /// Close a pane.
    Close {
        /// Target pane ID.
        pane: String,
    },

    /// Rename a pane (shown on the pane frame).
    Rename {
        /// Target pane ID.
        pane: String,
        /// New name.
        name: String,
    },

    /// Ping a pane and measure round-trip time.
    ///
    /// Sends a Ping envelope and waits for a Pong reply (up to --timeout
    /// seconds). Useful for checking if an agent is alive and responsive.
    ///
    /// Examples:
    ///   rz ping 3
    ///   rz ping terminal_1 --timeout 5
    Ping {
        /// Target pane ID.
        pane: String,
        /// Seconds to wait for a Pong reply.
        #[arg(long, default_value = "3")]
        timeout: u64,
    },

    /// Set a pane's color for visual identification.
    ///
    /// Examples:
    ///   rz color terminal_3 --bg "#003366"
    ///   rz color terminal_3 --fg "#00e000" --bg "#001a3a"
    ///   rz color terminal_3 --reset
    Color {
        /// Target pane ID.
        pane: String,
        /// Foreground color (hex).
        #[arg(long)]
        fg: Option<String>,
        /// Background color (hex).
        #[arg(long)]
        bg: Option<String>,
        /// Reset to terminal defaults.
        #[arg(long)]
        reset: bool,
    },

    /// Set a timer — hub delivers @@RZ: Timer message when it fires.
    ///
    /// Tickless: no polling. The hub calls set_timeout() and wakes you
    /// up with a Timer envelope when it expires.
    ///
    /// Examples:
    ///   rz timer 30 "check build"     # 30s timer with label
    ///   rz timer 5                     # 5s timer, empty label
    ///   rz timer --cancel 3            # cancel timer with id 3
    Timer {
        /// Delay in seconds.
        #[arg(required_unless_present = "cancel")]
        seconds: Option<f64>,
        /// Timer label (delivered in the Timer message).
        #[arg(default_value = "")]
        label: String,
        /// Cancel a pending timer by ID.
        #[arg(long)]
        cancel: Option<u64>,
    },
}

fn rz_path() -> String {
    std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "rz".into())
}

fn workspace_path() -> Result<std::path::PathBuf> {
    let session = std::env::var("ZELLIJ_SESSION_NAME")
        .map_err(|_| eyre::eyre!("ZELLIJ_SESSION_NAME not set — not inside zellij?"))?;
    Ok(std::path::PathBuf::from(format!("/tmp/rz-{session}")))
}

fn sender_id(from: Option<&str>) -> String {
    from.map(String::from)
        .or_else(|| zellij::own_pane_id().ok())
        .unwrap_or_else(|| "unknown".into())
}

/// Poll own scrollback for a reply referencing `msg_id`, with timeout.
fn wait_for_reply(msg_id: &str, timeout_secs: u64) -> Result<()> {
    let own = zellij::own_pane_id()?;
    let deadline = std::time::Instant::now()
        + std::time::Duration::from_secs(timeout_secs);
    loop {
        std::thread::sleep(std::time::Duration::from_millis(250));
        if std::time::Instant::now() >= deadline {
            bail!("timeout ({timeout_secs}s) — no reply to {msg_id}");
        }
        let scrollback = zellij::dump(&own)?;
        let messages = log::extract_messages(&scrollback);
        if let Some(reply) = messages.iter().rev().find(|m| {
            m.r#ref.as_deref() == Some(msg_id)
        }) {
            println!("{}", log::format_message(reply));
            return Ok(());
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Cmd::Id => {
            println!("{}", zellij::own_pane_id()?);
        }

        Cmd::Init => {
            let ws = workspace_path()?;
            std::fs::create_dir_all(ws.join("shared"))?;

            // Create project coordination files (idempotent — don't overwrite).
            let goals = ws.join("goals.md");
            if !goals.exists() {
                std::fs::write(&goals, "\
# Session Goals

> Agents: read this file when you start. Add sub-goals as you discover them.

## Goal
_Fill in the session's primary objective._

## Sub-goals
-

## Completed
-
")?;
            }

            let context = ws.join("context.md");
            if !context.exists() {
                std::fs::write(&context, "\
# Session Context

> Agents: append here, never delete. Prefix entries with the date.

## Decisions

## Discoveries

## Open Questions
-
")?;
            }

            let agents = ws.join("agents.md");
            if !agents.exists() {
                std::fs::write(&agents, "\
# Active Agents

> Agents: update your row when starting or finishing a task.

| Pane | Name | Current Task | Status |
|------|------|--------------|--------|
")?;
            }

            println!("{}", ws.display());
        }

        Cmd::Dir => {
            let ws = workspace_path()?;
            if !ws.exists() {
                bail!("workspace not initialized — run `rz init` first");
            }
            println!("{}", ws.display());
        }

        Cmd::Spawn {
            command,
            name,
            no_bootstrap,
            wait,
            prompt,
            color,
            args,
        } => {
            let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            let pane_id = zellij::spawn(&command, &arg_refs, name.as_deref())?;

            if let Some(n) = &name {
                let _ = zellij::rename(&pane_id, n);
            }
            if let Some(c) = &color {
                let _ = zellij::set_color(&pane_id, None, Some(c));
            }

            if !no_bootstrap {
                // Poll until the pane has output (process started) or timeout.
                let deadline = std::time::Instant::now()
                    + std::time::Duration::from_secs(wait);
                loop {
                    if std::time::Instant::now() >= deadline {
                        break;
                    }
                    if let Ok(out) = zellij::dump(&pane_id) {
                        if !out.trim().is_empty() {
                            break;
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }

                let msg = bootstrap::build(&pane_id, name.as_deref(), &rz_path())?;
                zellij::send(&pane_id, &msg)?;

                if let Some(task) = prompt {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    zellij::send(&pane_id, &task)?;
                }
            }

            println!("{pane_id}");
        }

        Cmd::Send { pane, message, raw, from, r#ref, wait } => {
            let pane = zellij::normalize_pane_id(&pane);
            if raw {
                if wait.is_some() {
                    bail!("--wait requires protocol mode (cannot use with --raw)");
                }
                zellij::send(&pane, &message)?;
            } else if zellij::hub_available() {
                // Route through the hub plugin via zellij pipe.
                let from = sender_id(from.as_deref());
                let mut args = vec![("target", pane.as_str()), ("from", from.as_str())];
                if let Some(r) = r#ref.as_deref() {
                    args.push(("ref", r));
                }
                let resp = zellij::pipe_to_hub("action=send", &args, Some(&message))?;

                if let Some(timeout_secs) = wait {
                    let msg_id = serde_json::from_str::<serde_json::Value>(&resp)
                        .ok()
                        .and_then(|v| v.get("data")?.get("message_id")?.as_str().map(String::from))
                        .unwrap_or_default();
                    if msg_id.is_empty() {
                        bail!("hub did not return a message_id");
                    }
                    wait_for_reply(&msg_id, timeout_secs)?;
                }
            } else {
                // Fallback: direct paste (no hub available).
                let mut envelope = Envelope::new(
                    sender_id(from.as_deref()),
                    MessageKind::Chat { text: message },
                );
                if let Some(r) = r#ref {
                    envelope = envelope.with_ref(r);
                }
                let msg_id = envelope.id.clone();
                zellij::send(&pane, &envelope.encode()?)?;

                if let Some(timeout_secs) = wait {
                    wait_for_reply(&msg_id, timeout_secs)?;
                }
            }
        }

        Cmd::Broadcast { message, raw } => {
            let from = sender_id(None);
            if !raw && zellij::hub_available() {
                // Route broadcast through the hub plugin.
                let resp = zellij::pipe_to_hub(
                    "action=broadcast",
                    &[("from", from.as_str())],
                    Some(&message),
                )?;
                let delivered = serde_json::from_str::<serde_json::Value>(&resp)
                    .ok()
                    .and_then(|v| v.get("data")?.get("delivered")?.as_u64())
                    .unwrap_or(0);
                eprintln!("broadcast to {delivered} panes (via hub)");
            } else {
                // Fallback: direct paste to each pane.
                let peers = zellij::list_pane_ids()?;
                let own = zellij::own_pane_id().ok();
                let mut sent = 0;

                for peer in &peers {
                    if own.as_deref() == Some(peer) {
                        continue;
                    }
                    if raw {
                        zellij::send(peer, &message)?;
                    } else {
                        let envelope = Envelope::new(
                            &from,
                            MessageKind::Chat { text: message.clone() },
                        );
                        zellij::send(peer, &envelope.encode()?)?;
                    }
                    sent += 1;
                }
                eprintln!("broadcast to {sent} panes");
            }
        }

        Cmd::List => {
            let panes = zellij::list_panes()?;
            let own = zellij::own_pane_id().ok();
            println!("{:<14} {:<16} {:<10} {:<20} {:<6} CWD",
                "PANE_ID", "TITLE", "TAB", "COMMAND", "EXIT");
            for p in &panes {
                if p.is_plugin {
                    continue;
                }
                let cmd = p.pane_command.as_deref().unwrap_or("-");
                let tab = p.tab_name.as_deref().unwrap_or("-");
                let cwd = p.pane_cwd.as_deref().unwrap_or("-");
                let pid = p.pane_id();
                let marker = if own.as_deref() == Some(&pid) { " *" } else { "" };
                let title = if p.title.is_empty() { "-" } else { &p.title };
                let exit = if p.exited {
                    p.exit_status
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "yes".into())
                } else {
                    "-".into()
                };
                println!("{:<14} {:<16} {:<10} {:<20} {:<6} {}{}",
                    pid, title, tab, cmd, exit, cwd, marker);
            }
        }

        Cmd::Status => {
            let panes = zellij::list_panes()?;
            let terminal_panes: Vec<_> = panes.into_iter().filter(|p| !p.is_plugin).collect();
            let summary = status::summarize(&terminal_panes, |id| zellij::dump(id).ok());
            print!("{}", status::format_summary(&summary));
        }

        Cmd::Dump { pane, last } => {
            let pane = zellij::normalize_pane_id(&pane);
            if let Some(n) = last {
                print!("{}", zellij::dump_last(&pane, n)?);
            } else {
                print!("{}", zellij::dump(&pane)?);
            }
        }

        Cmd::Log { pane, last } => {
            let pane = zellij::normalize_pane_id(&pane);
            let scrollback = zellij::dump(&pane)?;
            let mut messages = log::extract_messages(&scrollback);
            if let Some(n) = last {
                let skip = messages.len().saturating_sub(n);
                messages = messages.into_iter().skip(skip).collect();
            }
            for msg in &messages {
                println!("{}", log::format_message(msg));
            }
        }

        Cmd::Watch { pane, json } => {
            let pane = zellij::normalize_pane_id(&pane);
            let mut args = vec!["subscribe".to_string(), "--pane-id".to_string(), pane];
            if json {
                args.extend(["--format".to_string(), "json".to_string()]);
            }
            let status = std::process::Command::new("zellij")
                .args(&args)
                .status()?;
            if !status.success() {
                bail!("zellij subscribe exited with {status}");
            }
        }

        Cmd::Close { pane } => {
            let pane = zellij::normalize_pane_id(&pane);
            zellij::close(&pane)?;
        }

        Cmd::Rename { pane, name } => {
            let pane = zellij::normalize_pane_id(&pane);
            zellij::rename(&pane, &name)?;
        }

        Cmd::Ping { pane, timeout } => {
            let pane = zellij::normalize_pane_id(&pane);
            let own = zellij::own_pane_id()?;
            let from = sender_id(None);
            let envelope = Envelope::new(&from, MessageKind::Ping);
            let ping_id = envelope.id.clone();
            let sent = std::time::Instant::now();

            zellij::send(&pane, &envelope.encode()?)?;

            let deadline = sent + std::time::Duration::from_secs(timeout);
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if std::time::Instant::now() >= deadline {
                    println!("timeout ({timeout}s) — no pong from {pane}");
                    std::process::exit(1);
                }
                let scrollback = zellij::dump(&own)?;
                let messages = log::extract_messages(&scrollback);
                let got_pong = messages.iter().any(|m| {
                    matches!(m.kind, MessageKind::Pong)
                        && m.r#ref.as_deref() == Some(&ping_id)
                });
                if got_pong {
                    let rtt = sent.elapsed();
                    println!("pong from {pane} in {:.1}ms", rtt.as_secs_f64() * 1000.0);
                    break;
                }
            }
        }

        Cmd::Color { pane, fg, bg, reset } => {
            let pane = zellij::normalize_pane_id(&pane);
            if reset {
                zellij::reset_color(&pane)?;
            } else {
                zellij::set_color(&pane, fg.as_deref(), bg.as_deref())?;
            }
        }

        Cmd::Timer { seconds, label, cancel } => {
            if !zellij::hub_available() {
                bail!("timer requires the rz-hub plugin (RZ_HUB=1 not set)");
            }

            if let Some(timer_id) = cancel {
                let resp = zellij::pipe_to_hub(
                    "action=cancel_timer",
                    &[("timer_id", &timer_id.to_string())],
                    None,
                )?;
                let parsed = serde_json::from_str::<serde_json::Value>(&resp).ok();
                if parsed.as_ref().and_then(|v| v.get("ok")?.as_bool()) == Some(true) {
                    eprintln!("cancelled timer {timer_id}");
                } else {
                    let err = parsed
                        .and_then(|v| v.get("error")?.as_str().map(String::from))
                        .unwrap_or_else(|| "unknown error".into());
                    bail!("cancel failed: {err}");
                }
            } else {
                let seconds = seconds.unwrap();
                let target = sender_id(None);
                let secs_str = seconds.to_string();
                let resp = zellij::pipe_to_hub(
                    "action=timer",
                    &[("target", target.as_str()), ("seconds", &secs_str)],
                    Some(&label),
                )?;
                let timer_id = serde_json::from_str::<serde_json::Value>(&resp)
                    .ok()
                    .and_then(|v| v.get("data")?.get("timer_id")?.as_u64());
                match timer_id {
                    Some(id) => println!("timer {id} set for {seconds}s"),
                    None => {
                        let err = serde_json::from_str::<serde_json::Value>(&resp)
                            .ok()
                            .and_then(|v| v.get("error")?.as_str().map(String::from))
                            .unwrap_or_else(|| "unknown error".into());
                        bail!("timer failed: {err}");
                    }
                }
            }
        }
    }

    Ok(())
}
