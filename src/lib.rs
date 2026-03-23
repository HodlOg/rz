//! `rz` — inter-agent communication over Zellij.
//!
//! Uses `zellij action write-chars --pane-id` for direct pane-to-pane
//! messaging. No files, no sockets, no focus switching.

pub mod bootstrap;
pub mod protocol;
pub mod zellij;

pub use protocol::{Envelope, MessageKind, SENTINEL};
