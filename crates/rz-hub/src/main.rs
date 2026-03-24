use std::collections::BTreeMap;
use zellij_tile::prelude::*;

mod registry;
mod router;

pub struct PendingTimer {
    pub id: u64,
    pub pane_id: u32,
    pub label: String,
    pub seconds: f64,
}

struct RzHub {
    registry: registry::AgentRegistry,
    plugin_id: u32,
    dirty: bool,
    timers: Vec<PendingTimer>,
    next_timer_id: u64,
}

impl Default for RzHub {
    fn default() -> Self {
        Self {
            registry: registry::AgentRegistry::default(),
            plugin_id: 0,
            dirty: false,
            timers: Vec::new(),
            next_timer_id: 1,
        }
    }
}

register_plugin!(RzHub);

impl ZellijPlugin for RzHub {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::WriteToStdin,
            PermissionType::ReadCliPipes,
        ]);

        subscribe(&[
            EventType::PaneUpdate,
            EventType::PermissionRequestResult,
            EventType::Timer,
        ]);

        let ids = get_plugin_ids();
        self.plugin_id = ids.plugin_id;

        if configuration.get("visible").map(|v| v.as_str()) != Some("true") {
            hide_self();
        }
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PaneUpdate(manifest) => {
                self.registry
                    .update_from_pane_manifest(&manifest, self.plugin_id);
                self.dirty = true;
            }
            Event::Timer(elapsed) => {
                // Find the timer(s) whose delay matches the elapsed value.
                // set_timeout fires with the exact seconds value we passed.
                let elapsed_secs = elapsed as f64;
                let mut fired = Vec::new();
                self.timers.retain(|t| {
                    if (t.seconds - elapsed_secs).abs() < 0.01 {
                        fired.push((t.pane_id, t.label.clone()));
                        false
                    } else {
                        true
                    }
                });
                for (pane_id, label) in fired {
                    router::deliver_timer(pane_id, &label);
                }
            }
            Event::PermissionRequestResult(_) => {}
            _ => {}
        }
        self.dirty
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        if pipe_message.name != "rz" {
            return false;
        }
        router::handle_pipe(
            &mut self.registry,
            &pipe_message,
            &mut self.timers,
            &mut self.next_timer_id,
        );
        self.dirty = matches!(
            pipe_message.args.get("action").map(|s| s.as_str()),
            Some("register" | "unregister")
        );
        self.dirty
    }

    fn render(&mut self, _rows: usize, _cols: usize) {
        self.dirty = false;
        // Dashboard rendering skipped in v1.
    }
}
