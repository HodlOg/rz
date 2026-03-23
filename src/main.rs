//! `rz` — inter-agent messaging over Zellij.

use clap::{Parser, Subcommand};
use eyre::{Result, bail};

use rz_cli::protocol::{Envelope, MessageKind};
use rz_cli::{bootstrap, zellij};

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
    ///   rz send terminal_3 "research this topic"
    ///   rz send --raw terminal_3 "ls -la"
    ///   rz send --from orchestrator terminal_3 "do this"
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

    /// Dump a pane's full scrollback to stdout.
    Dump {
        /// Target pane ID.
        pane: String,
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
}

fn rz_path() -> String {
    std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "rz".into())
}

fn sender_id(from: Option<&str>) -> String {
    from.map(String::from)
        .or_else(|| zellij::own_pane_id().ok())
        .unwrap_or_else(|| "unknown".into())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Cmd::Id => {
            println!("{}", zellij::own_pane_id()?);
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
                std::thread::sleep(std::time::Duration::from_secs(wait));
                let msg = bootstrap::build(&pane_id, name.as_deref(), &rz_path())?;
                zellij::send(&pane_id, &msg)?;

                if let Some(task) = prompt {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    zellij::send(&pane_id, &task)?;
                }
            }

            println!("{pane_id}");
        }

        Cmd::Send { pane, message, raw, from } => {
            if raw {
                zellij::send(&pane, &message)?;
            } else {
                let envelope = Envelope::new(
                    sender_id(from.as_deref()),
                    MessageKind::Chat { text: message },
                );
                zellij::send(&pane, &envelope.encode()?)?;
            }
        }

        Cmd::Broadcast { message, raw } => {
            let from = sender_id(None);
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

        Cmd::List => {
            let panes = zellij::list_panes()?;
            println!("{:<14} {:<10} {:<20} {:<6} CWD",
                "PANE_ID", "TAB", "COMMAND", "EXIT");
            for p in &panes {
                if p.is_plugin {
                    continue;
                }
                let cmd = p.pane_command.as_deref().unwrap_or("-");
                let tab = p.tab_name.as_deref().unwrap_or("-");
                let cwd = p.pane_cwd.as_deref().unwrap_or("-");
                let exit = if p.exited {
                    p.exit_status
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "yes".into())
                } else {
                    "-".into()
                };
                println!("{:<14} {:<10} {:<20} {:<6} {}",
                    p.pane_id(), tab, cmd, exit, cwd);
            }
        }

        Cmd::Dump { pane } => {
            print!("{}", zellij::dump(&pane)?);
        }

        Cmd::Watch { pane, json } => {
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
            zellij::close(&pane)?;
        }

        Cmd::Rename { pane, name } => {
            zellij::rename(&pane, &name)?;
        }

        Cmd::Color { pane, fg, bg, reset } => {
            if reset {
                zellij::reset_color(&pane)?;
            } else {
                zellij::set_color(&pane, fg.as_deref(), bg.as_deref())?;
            }
        }
    }

    Ok(())
}
