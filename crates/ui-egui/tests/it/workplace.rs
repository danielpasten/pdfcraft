//! The workplace build (`PdfCraftApp::workplace`): staff see only finished tools, and no AI
//! placeholders, developer credits, project links or update check. Each test also checks the
//! default build, so it fails if the switch stops doing anything.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_engine::catalog::{Availability, TOOL_GROUPS};
use pdfcraft_ui_egui::{Dialog, LeftPanel, PdfCraftApp};

fn harness(workplace: bool, setup: impl FnOnce(&mut PdfCraftApp) + 'static) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.workplace = workplace;
        setup(&mut app);
        app
    });
    h.run_steps(4);
    h
}

fn shows(h: &Harness<'static, PdfCraftApp>, text: &str) -> bool {
    h.query_all_by(|n| n.label().is_some_and(|l| l.contains(text))).count() > 0
}

// The home screen's privacy note is plain text, which the accessibility tree doesn't carry, so
// only its GitHub button is checked here.
#[test]
fn title_bar_has_no_project_link() {
    assert!(shows(&harness(false, |_| {}), "GitHub"), "the default build has the GitHub button");
    assert!(!shows(&harness(true, |_| {}), "GitHub"), "no GitHub button");
}

#[test]
fn all_tools_lists_no_ai_placeholders() {
    let expanded = |app: &mut PdfCraftApp| {
        app.left_open = true;
        app.left = LeftPanel::AllTools;
        app.all_tools_expanded = true;
    };
    assert!(shows(&harness(false, expanded), "AI assistant"));
    assert!(!shows(&harness(true, expanded), "AI assistant"));
}

#[test]
fn tool_panels_list_only_finished_tools() {
    // Labels of entries that don't work yet, and that no finished entry of the group shares.
    let unfinished = |id: &str| -> Vec<&'static str> {
        let Some(g) = TOOL_GROUPS.iter().find(|g| g.id == id) else { return Vec::new() };
        let items: Vec<_> = g.sections.iter().flat_map(|s| s.items.iter()).collect();
        let ready: Vec<_> = items.iter().filter(|i| i.availability == Availability::Ready).map(|i| i.label).collect();
        items.iter().filter(|i| i.availability != Availability::Ready && !ready.contains(&i.label)).map(|i| i.label).collect()
    };
    let mut default_showed_one = false;
    for g in TOOL_GROUPS.iter().filter(|g| g.id != "ai" && !unfinished(g.id).is_empty()) {
        let id = g.id;
        let open = move |app: &mut PdfCraftApp| {
            app.left_open = true;
            app.left = LeftPanel::Tool(id);
        };
        let h = harness(false, open);
        default_showed_one |= unfinished(id).iter().any(|l| shows(&h, l)) || shows(&h, "milestone");
        let h = harness(true, open);
        for label in unfinished(id) {
            assert!(h.query_all_by_label(label).count() == 0, "{id}: unfinished {label:?} is shown");
        }
        assert!(!shows(&h, "milestone"), "{id}: a milestone is mentioned");
    }
    assert!(default_showed_one, "the default build shows unfinished tools somewhere");
}

#[test]
fn about_shows_no_credits_tabs_or_links() {
    let about = |app: &mut PdfCraftApp| app.dialog = Some(Dialog::About);
    let h = harness(false, about);
    assert!(h.query_all_by_label("Models").count() > 0 && h.query_all_by_label("Contributors").count() > 0);
    let h = harness(true, about);
    for gone in ["Models", "Contributors"] {
        assert_eq!(h.query_all_by_label(gone).count(), 0, "{gone} tab");
    }
    assert!(!shows(&h, "GitHub") && !shows(&h, "bootstrap"), "no project link or developer credits");
    assert!(h.query_all_by_label("PdfCraft").count() >= 2, "still the icon and the name");
}

#[test]
fn project_link_and_update_check_do_nothing() {
    let mut h = harness(true, |_| {});
    for id in ["help.github", "help.check_updates"] {
        assert!(!h.state_mut().execute(id), "{id} runs");
    }
    h.run_steps(2);
    assert_eq!(h.state().last_opened_url, None, "nothing was opened");
    let mut h = harness(false, |_| {});
    assert!(h.state_mut().execute("help.github"), "the default build opens the repository");
}

const ONE_PAGE: &[u8] = b"%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF";
const MY_APPS: &str = "https://launcher.myapps.microsoft.com/api/signin/";

fn with_document(workplace: bool) -> Harness<'static, PdfCraftApp> {
    harness(workplace, |app| {
        app.open_bytes("one.pdf", None, ONE_PAGE.to_vec()).unwrap();
    })
}

fn signing_area(h: &Harness<'static, PdfCraftApp>) -> bool {
    matches!(h.state().quick_tool, pdfcraft_ui_egui::QuickTool::SignArea { .. })
}

#[test]
fn signing_asks_to_use_adobe_sign_first() {
    let mut h = with_document(true);
    assert!(h.state_mut().execute("sign.digital"));
    h.run_steps(3);
    assert!(!signing_area(&h), "signing waits for an answer");
    h.get_by_label("Open Adobe Sign").click();
    h.run_steps(3);
    assert!(h.state().last_opened_url.as_deref().is_some_and(|u| u.starts_with(MY_APPS)), "{:?}", h.state().last_opened_url);
    assert!(!signing_area(&h) && h.query_by_label("Open Adobe Sign").is_none(), "the prompt is gone and nothing was signed here");
}

#[test]
fn sign_here_instead_signs_and_stops_asking() {
    let mut h = with_document(true);
    h.state_mut().execute("sign.certify");
    h.run_steps(3);
    h.get_by_label("Sign here instead").click();
    h.run_steps(3);
    assert!(signing_area(&h), "the held command ran");
    assert_eq!(h.state().last_opened_url, None);
    h.state_mut().quick_tool = pdfcraft_ui_egui::QuickTool::Select;
    h.state_mut().execute("sign.digital");
    h.run_steps(3);
    assert!(signing_area(&h) && h.query_by_label("Open Adobe Sign").is_none(), "no second prompt until restart");
}

#[test]
fn cancel_signs_nothing_and_the_default_build_never_asks() {
    let mut h = with_document(true);
    h.state_mut().execute("sign.digital");
    h.run_steps(3);
    h.get_by_label("Cancel").click();
    h.run_steps(3);
    assert!(!signing_area(&h) && h.state().last_opened_url.is_none());
    let mut h = with_document(false);
    h.state_mut().execute("sign.digital");
    h.run_steps(3);
    assert!(signing_area(&h) && h.query_by_label("Open Adobe Sign").is_none(), "upstream behaviour");
}

#[test]
fn fill_and_sign_offers_request_signatures() {
    let open = |app: &mut PdfCraftApp| {
        app.left_open = true;
        app.left = LeftPanel::Tool("fill_sign");
    };
    assert!(harness(false, open).query_by_label("Request signatures (Adobe Sign)").is_none());
    let mut h = harness(true, open);
    h.get_by_label("Request signatures (Adobe Sign)").click();
    h.run_steps(2);
    assert!(h.state().last_opened_url.as_deref().is_some_and(|u| u.starts_with(MY_APPS)));
}
