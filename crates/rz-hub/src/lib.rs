use std::collections::BTreeMap;
use zellij_tile::prelude::*;

mod registry;
mod router;

struct RzHub {
    registry: registry::AgentRegistry,
    plugin_id: u32,
    dirty: bool,
}

impl Default for RzHub {
    fn default() -> Self {
        Self {
            registry: registry::AgentRegistry::default(),
            plugin_id: 0,
            dirty: false,
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
            Event::PermissionRequestResult(_) => {}
            _ => {}
        }
        self.dirty
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        if pipe_message.name != "rz" {
            return false;
        }
        router::handle_pipe(&mut self.registry, &pipe_message);
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
