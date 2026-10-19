//! Settings > Customize: html.css editor and shortcut editor.

use crate::*;

pub(crate) struct CustomizePage {
    pub(crate) css_buffer: gtk::TextBuffer,
}

pub(crate) fn build_customize_page(settings_stack: &gtk::Stack) -> CustomizePage {
    // --- Page: Customize (html.css editor) ---
    let page_css = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page_css.set_vexpand(true);

    let css_buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
    let css_view = gtk::TextView::with_buffer(&css_buffer);
    css_view.add_css_class("css-editor");
    css_view.set_monospace(true);
    css_view.set_wrap_mode(gtk::WrapMode::Word);
    css_view.set_left_margin(10);
    css_view.set_right_margin(10);
    css_view.set_top_margin(8);
    css_view.set_bottom_margin(8);
    let css_scroll = gtk::ScrolledWindow::new();
    css_scroll.add_css_class("css-editor-frame");
    css_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    css_scroll.set_vexpand(true);
    css_scroll.set_min_content_height(300);
    css_scroll.set_child(Some(&css_view));
    page_css.append(&css_scroll);

    let css_save_btn = gtk::Button::with_label(tr("Save"));
    css_save_btn.add_css_class("settings-option-item");
    css_save_btn.add_css_class("active");
    css_save_btn.set_halign(gtk::Align::Start);
    page_css.append(&css_save_btn);

    {
        let buf = css_buffer.clone();
        css_save_btn.connect_clicked(move |b| save_css_editor(&buf, b));
    }
    {
        let buf = css_buffer.clone();
        let btn = css_save_btn.clone();
        let kc = gtk::EventControllerKey::new();
        kc.set_propagation_phase(gtk::PropagationPhase::Capture);
        kc.connect_key_pressed(move |_, key, _, state| {
            if state.contains(gdk::ModifierType::CONTROL_MASK) && key.to_lower() == gdk::Key::s {
                save_css_editor(&buf, &btn);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        css_view.add_controller(kc);
    }

    // --- Customize > Shortcuts subtab ---
    let sc_pending: Rc<RefCell<ShortcutConfig>> = Rc::new(RefCell::new(ShortcutConfig::default()));
    let sc_page = gtk::Box::new(gtk::Orientation::Vertical, 6);
    sc_page.set_vexpand(true);

    let sc_prio = gtk::CheckButton::with_label(tr("Rail shortcuts win over website shortcuts"));
    sc_prio.set_active(true);
    let sc_prio_hint = gtk::Label::new(Some(
        tr("Unchecked: when a page uses the same keys, the page wins and Rail stays out of the way."),
    ));
    sc_prio_hint.add_css_class("settings-card-desc");
    sc_prio_hint.set_xalign(0.0);
    sc_prio_hint.set_wrap(true);
    sc_page.append(&sc_prio);
    sc_page.append(&sc_prio_hint);

    let sc_rows = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let sc_scroll = gtk::ScrolledWindow::new();
    sc_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    sc_scroll.set_vexpand(true);
    sc_scroll.set_min_content_height(260);
    sc_scroll.set_child(Some(&sc_rows));
    sc_page.append(&sc_scroll);
    // Host for the single shared key-picker popover (not one per row).
    let picker_host = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    picker_host.set_halign(gtk::Align::End);
    sc_page.append(&picker_host);

    let sc_status = gtk::Label::new(None);
    sc_status.add_css_class("settings-card-desc");
    sc_status.set_xalign(0.0);
    sc_status.set_wrap(true);
    sc_page.append(&sc_status);

    let sc_reset_all = gtk::Button::with_label(tr("Reset all to defaults"));
    sc_reset_all.add_css_class("settings-option-item");
    sc_reset_all.set_halign(gtk::Align::Start);
    sc_page.append(&sc_reset_all);

    let is_syncing = Rc::new(Cell::new(false));
    let sc_ui = ScUi {
        rows: sc_rows.clone(),
        pending: sc_pending.clone(),
        status: sc_status.clone(),
        prio: sc_prio.clone(),
        is_syncing: is_syncing.clone(),
        picker_host: picker_host.clone(),
    };
    {
        let ui = sc_ui.clone();
        sc_prio.connect_toggled(move |c| {
            if ui.is_syncing.get() {
                return;
            }
            // Master switch is boss: ON → every shortcut on; OFF → every (non-locked) off
            let rail_wins = c.is_active();
            {
                let mut p = ui.pending.borrow_mut();
                p.site_wins = !rail_wins;
                p.disabled.clear();
                if !rail_wins {
                    for def in SHORTCUT_DEFS {
                        if !sc_is_locked(def.id) {
                            p.disabled.insert(def.id.to_string());
                        }
                    }
                }
            }
            sc_commit(&ui);
            // Idle rebuild — never block the click handler with a full widget tree rebuild.
            sc_refresh(&ui);
        });
    }
    {
        let ui = sc_ui.clone();
        let prio = sc_prio.clone();
        sc_page.connect_map(move |_| {
            *ui.pending.borrow_mut() = SHORTCUTS.with(|c| c.borrow().clone());
            let all_on = {
                let c = ui.pending.borrow();
                !c.site_wins && SHORTCUT_DEFS.iter().all(|d| sc_enabled(&c, d.id))
            };
            ui.is_syncing.set(true);
            prio.set_active(all_on);
            // Leave is_syncing true briefly — cleared at end of sc_build via timeout.
            ui.status.set_text("");
            // Only build if the list is empty (first open). Avoid re-map rebuild storms.
            if ui.rows.first_child().is_none() {
                // Defer off the click stack so Settings stays responsive.
                let ui2 = ui.clone();
                glib::idle_add_local_once(move || sc_build(&ui2));
            } else {
                ui.is_syncing.set(false);
            }
        });
    }
    {
        let ui = sc_ui.clone();
        sc_reset_all.connect_clicked(move |_| {
            {
                let mut c = ui.pending.borrow_mut();
                c.overrides.clear();
                c.disabled.clear();
                c.site_wins = false;
            }
            ui.is_syncing.set(true);
            ui.prio.set_active(true);
            ui.is_syncing.set(false);
            sc_commit(&ui);
            ui.status.set_text(tr("All shortcuts are back to their defaults."));
            sc_build(&ui);
        });
    }

    // --- Customize sub-tab bar ---
    let customize_stack = gtk::Stack::new();
    customize_stack.set_vexpand(true);
    customize_stack.add_named(&page_css, Some("html"));
    customize_stack.add_named(&sc_page, Some("shortcuts"));

    let sub_bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let sub_html = gtk::Button::with_label(tr("HTML Defaults"));
    sub_html.add_css_class("settings-option-item");
    sub_html.add_css_class("active");
    let sub_sc = gtk::Button::with_label(tr("Shortcuts"));
    sub_sc.add_css_class("settings-option-item");
    sub_bar.append(&sub_html);
    sub_bar.append(&sub_sc);
    {
        let stack = customize_stack.clone();
        let other = sub_sc.clone();
        sub_html.connect_clicked(move |b| {
            stack.set_visible_child_name("html");
            b.add_css_class("active");
            other.remove_css_class("active");
        });
    }
    {
        let stack = customize_stack.clone();
        let other = sub_html.clone();
        sub_sc.connect_clicked(move |b| {
            stack.set_visible_child_name("shortcuts");
            b.add_css_class("active");
            other.remove_css_class("active");
        });
    }

    let page_customize = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page_customize.add_css_class("settings-content-page");
    page_customize.set_vexpand(true);
    page_customize.append(&sub_bar);
    page_customize.append(&customize_stack);
    settings_stack.add_named(&page_customize, Some("customize"));

    CustomizePage { css_buffer }
}
