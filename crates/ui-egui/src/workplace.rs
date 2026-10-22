//! The workplace build ([`PdfCraftApp::workplace`]): what a managed work computer leaves out.
//!
//! Staff see only tools that work today. Planned tools, their milestone tags and the "AI
//! assistant" placeholders (no provider can be configured) are hidden, and so are the project
//! links and the update check: IT installs and updates the app. Nothing is removed from the
//! catalogue or the command registry, so automation and the tests see the same app.

use pdfcraft_engine::catalog::{Availability, ToolGroup, ToolItem};

use crate::PdfCraftApp;

/// Registry commands a workplace build leaves out of its menus, palette and dispatch.
const HIDDEN_COMMANDS: &[&str] = &["help.github", "help.check_updates"];

impl PdfCraftApp {
    /// Whether the registry command `id` is left out of this build.
    pub(crate) fn command_hidden(&self, id: &str) -> bool {
        self.workplace && HIDDEN_COMMANDS.contains(&id)
    }

    /// Whether a tool entry is shown: in a workplace build, only one that works today.
    pub(crate) fn tool_item_shown(&self, item: &ToolItem) -> bool {
        !self.workplace || item.availability == Availability::Ready
    }

    /// Whether a tool group is shown: in a workplace build, only one with an entry that works.
    pub(crate) fn tool_group_shown(&self, group: &ToolGroup) -> bool {
        !self.workplace || group.sections.iter().flat_map(|s| s.items.iter()).any(|i| i.availability == Availability::Ready)
    }

    /// How a shown group presents itself. A workplace build shows only what works, so a group
    /// still planned as a whole (Edit a PDF, with most of its entries ready) carries no milestone.
    pub(crate) fn group_availability(&self, group: &ToolGroup) -> Availability {
        if self.workplace { Availability::Ready } else { group.availability }
    }
}

#[cfg(test)]
mod tests {
    use pdfcraft_engine::catalog::{Availability, TOOL_GROUPS};

    use crate::PdfCraftApp;

    #[test]
    fn a_workplace_build_shows_only_what_works() {
        let mut app = PdfCraftApp::new();
        assert!(TOOL_GROUPS.iter().all(|g| app.tool_group_shown(g)), "the default build shows the whole catalogue");
        assert!(!app.command_hidden("help.check_updates"));
        app.workplace = true;
        let shown: Vec<_> = TOOL_GROUPS.iter().filter(|g| app.tool_group_shown(g)).collect();
        assert!(!shown.is_empty() && shown.len() < TOOL_GROUPS.len(), "some groups have nothing finished");
        assert!(TOOL_GROUPS.iter().any(|g| g.id == "ai") && shown.iter().all(|g| g.id != "ai"), "no AI placeholders");
        for g in shown {
            assert_eq!(app.group_availability(g), Availability::Ready, "{}", g.id);
            for i in g.sections.iter().flat_map(|s| s.items.iter()).filter(|i| app.tool_item_shown(i)) {
                assert_eq!(i.availability, Availability::Ready, "{}", i.command);
            }
        }
        assert!(app.command_hidden("help.github") && app.command_hidden("help.check_updates"));
        assert!(!app.command_hidden("file.open"));
    }
}
