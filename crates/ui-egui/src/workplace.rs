//! The workplace build ([`PdfCraftApp::workplace`]): what a managed work computer leaves out.
//!
//! Staff see only tools that work today. Planned tools, their milestone tags and the "AI
//! assistant" placeholders (no provider can be configured) are hidden, and so are the project
//! links and the update check: IT installs and updates the app. Nothing is removed from the
//! catalogue or the command registry, so automation and the tests see the same app.
//!
//! Signing follows Women for Women International's procedure: official signatures, and requests
//! for signatures from people outside the organization, go through Adobe Sign. Every way of
//! signing in the app first offers to open Adobe Sign (through Microsoft My Apps), or to sign here
//! instead; automation (scripts, MCP, the control channel) signs without asking.

use pdfcraft_engine::catalog::{Availability, ToolGroup, ToolItem};

use crate::PdfCraftApp;

/// Registry commands a workplace build leaves out of its menus, palette and dispatch.
const HIDDEN_COMMANDS: &[&str] = &["help.github", "help.check_updates"];

/// Adobe Sign, opened through Microsoft My Apps: the link signs in with the work account.
pub(crate) const ADOBE_SIGN: &str =
    "https://launcher.myapps.microsoft.com/api/signin/affdfd99-be25-4f7a-aaef-cc91beecf63b?tenantId=75184fe4-418a-4d4f-90cf-d827dfe17e5c";

/// Registry commands that sign: Fill & Sign signatures and initials, and certificate signatures.
const SIGNING_COMMANDS: &[&str] = &["sign.fill.signature", "sign.fill.initials", "sign.digital", "sign.certify", "sign.certify_invisible"];

/// A signing action waiting for the answer to the Adobe Sign prompt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PendingSign {
    Command(&'static str),
    /// Sign the unsigned signature field `field` (shown on `page`).
    Field {
        page: usize,
        field: String,
    },
}

/// The answer to the Adobe Sign prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SignChoice {
    AdobeSign,
    SignHere,
    Cancel,
}

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

    /// Holds a signing action for the Adobe Sign prompt and returns true, or returns false when it
    /// can go ahead: not a workplace build, not a signing action, or "Sign here instead" was
    /// already chosen since the app started.
    pub(crate) fn hold_for_adobe_sign(&mut self, pending: PendingSign) -> bool {
        let signing = match &pending {
            PendingSign::Command(id) => SIGNING_COMMANDS.contains(id),
            PendingSign::Field { .. } => true,
        };
        if !self.workplace || !signing || self.sign_here_ok {
            return false;
        }
        self.pending_sign = Some(pending);
        true
    }

    /// Sign the unsigned signature field `field` on `page`, after the Adobe Sign prompt in a
    /// workplace build.
    pub(crate) fn sign_field(&mut self, page: usize, field: String) {
        if !self.hold_for_adobe_sign(PendingSign::Field { page, field: field.clone() }) {
            self.start_signing(page, None, Some(field), None);
        }
    }

    /// Act on the answer to the Adobe Sign prompt. "Sign here instead" stops the prompt until the
    /// app restarts, so it doesn't stand in the way of someone who has decided.
    pub(crate) fn resolve_adobe_sign(&mut self, choice: SignChoice) {
        let Some(pending) = self.pending_sign.take() else { return };
        match choice {
            SignChoice::AdobeSign => self.open_url(ADOBE_SIGN),
            SignChoice::SignHere => {
                self.sign_here_ok = true;
                match pending {
                    PendingSign::Command(id) => {
                        self.execute(id);
                    }
                    PendingSign::Field { page, field } => self.start_signing(page, None, Some(field), None),
                }
            }
            SignChoice::Cancel => {}
        }
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
