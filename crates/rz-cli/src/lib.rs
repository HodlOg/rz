//! `rz` — inter-agent communication over Zellij.
//!
//! Uses `zellij action write-chars --pane-id` for direct pane-to-pane
//! messaging. No files, no sockets, no focus switching.

pub mod bootstrap;
pub mod log;
pub mod status;
pub mod zellij;

/// Re-export the protocol crate so downstream code can use `rz_cli::protocol::*`.
pub use rz_protocol as protocol;

pub use rz_protocol::{Envelope, MessageKind, SENTINEL};
