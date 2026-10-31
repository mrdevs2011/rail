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
    setup_css_tags(&css_buffer);
    css_buffer.set_text(&load_html_css());

    let gutter_buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
    let gutter_view = gtk::TextView::with_buffer(&gutter_buffer);
    gutter_view.add_css_class("css-editor-gutter");
    gutter_view.set_monospace(true);
    gutter_view.set_editable(false);
    gutter_view.set_can_focus(false);
    gutter_view.set_can_target(false);
    gutter_view.set_wrap_mode(gtk::WrapMode::None);
    gutter_view.set_left_margin(8);
    gutter_view.set_right_margin(8);
    gutter_view.set_top_margin(8);
    gutter_view.set_bottom_margin(8);

    let css_view = gtk::TextView::with_buffer(&css_buffer);
    css_view.add_css_class("css-editor");
    css_view.set_monospace(true);
    css_view.set_wrap_mode(gtk::WrapMode::None);
    css_view.set_left_margin(12);
    css_view.set_right_margin(12);
    css_view.set_top_margin(8);
    css_view.set_bottom_margin(8);
    css_view.set_hexpand(true);
    css_view.set_vexpand(true);

    let editor_hbox = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    editor_hbox.set_hexpand(true);
    editor_hbox.set_vexpand(true);
    editor_hbox.append(&gutter_view);
    editor_hbox.append(&css_view);

    // Editor actions stay above the code so Reset is always immediately discoverable.
    let css_toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    css_toolbar.set_homogeneous(false);
    let css_reset_btn = gtk::Button::with_label(tr("Reset to default"));
    css_reset_btn.add_css_class("settings-option-item");
    css_reset_btn.set_tooltip_text(Some(tr("Restore the built-in developer CSS template")));
    let css_save_btn = gtk::Button::with_label(tr("Save"));
    css_save_btn.add_css_class("settings-option-item");
    css_save_btn.add_css_class("active");
    css_toolbar.append(&css_reset_btn);
    css_toolbar.append(&css_save_btn);
    page_css.append(&css_toolbar);

    {
        let buf = css_buffer.clone();
        let save = css_save_btn.clone();
        css_reset_btn.connect_clicked(move |reset| {
            buf.set_text(DEFAULT_HTML_CSS);
            save_css_editor(&buf, &save);
            reset.set_label(tr("Reset"));
            let b = reset.clone();
            glib::timeout_add_local_once(Duration::from_millis(1500), move || {
                b.set_label(tr("Reset to default"));
            });
        });
    }

    let css_scroll = gtk::ScrolledWindow::new();
    css_scroll.add_css_class("css-editor-frame");
    css_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    css_scroll.set_vexpand(true);
    css_scroll.set_min_content_height(480);
    css_scroll.set_child(Some(&editor_hbox));
    page_css.append(&css_scroll);

    // Initial gutter and syntax highlight
    update_gutter(&css_buffer, &gutter_buffer);
    highlight_css(&css_buffer);

    let is_highlighting = Rc::new(Cell::new(false));
    {
        let c_buf = css_buffer.clone();
        let g_buf = gutter_buffer.clone();
        let hl_flag = is_highlighting.clone();
        css_buffer.connect_changed(move |_| {
            if hl_flag.get() {
                return;
            }
            hl_flag.set(true);
            update_gutter(&c_buf, &g_buf);
            highlight_css(&c_buf);
            hl_flag.set(false);
        });
    }
    {
        let c_buf = css_buffer.clone();
        let g_buf = gutter_buffer.clone();
        let hl_flag = is_highlighting.clone();
        page_css.connect_map(move |_| {
            if c_buf.char_count() == 0 {
                hl_flag.set(true);
                c_buf.set_text(&load_html_css());
                update_gutter(&c_buf, &g_buf);
                highlight_css(&c_buf);
                hl_flag.set(false);
            }
        });
    }

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

    let sc_rows = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let sc_scroll = gtk::ScrolledWindow::new();
    sc_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
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
        is_syncing: is_syncing.clone(),
        picker_host: picker_host.clone(),
    };
    {
        let ui = sc_ui.clone();
        sc_page.connect_map(move |_| {
            *ui.pending.borrow_mut() = SHORTCUTS.with(|c| c.borrow().clone());
            // sc_build holds the sync guard while rows are rebuilt, then releases it.
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
            }
            sc_commit(&ui);
            ui.status
                .set_text(tr("All shortcuts are back to their defaults."));
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

fn setup_css_tags(buf: &gtk::TextBuffer) {
    let table = buf.tag_table();
    let comment = gtk::TextTag::builder()
        .name("css_comment")
        .foreground("#71717a")
        .style(gtk::pango::Style::Italic)
        .build();
    let selector = gtk::TextTag::builder()
        .name("css_selector")
        .foreground("#f0f0f0")
        .weight(700)
        .build();
    let prop = gtk::TextTag::builder()
        .name("css_prop")
        .foreground("#c8c8c8")
        .build();
    let val = gtk::TextTag::builder()
        .name("css_val")
        .foreground("#a8a8a8")
        .build();
    let str_tag = gtk::TextTag::builder()
        .name("css_str")
        .foreground("#e2e2e2")
        .build();
    let num_tag = gtk::TextTag::builder()
        .name("css_num")
        .foreground("#909090")
        .build();
    let punct = gtk::TextTag::builder()
        .name("css_punct")
        .foreground("#bdbdbd")
        .build();

    let _ = table.add(&comment);
    let _ = table.add(&selector);
    let _ = table.add(&prop);
    let _ = table.add(&val);
    let _ = table.add(&str_tag);
    let _ = table.add(&num_tag);
    let _ = table.add(&punct);
}

fn update_gutter(css_buf: &gtk::TextBuffer, gutter_buf: &gtk::TextBuffer) {
    let lines = css_buf.line_count().max(1);
    let mut s = String::with_capacity(lines as usize * 5);
    for i in 1..=lines {
        use std::fmt::Write;
        let _ = writeln!(s, "{:>4}", i);
    }
    if s.ends_with('\n') {
        s.pop();
    }
    gutter_buf.set_text(&s);
}

fn highlight_css(buf: &gtk::TextBuffer) {
    let (start_iter, end_iter) = buf.bounds();
    buf.remove_all_tags(&start_iter, &end_iter);
    let text = buf.text(&start_iter, &end_iter, false).to_string();
    if text.is_empty() {
        return;
    }

    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_block = false;
    let mut in_value = false;

    while i < len {
        // Comments /* ... */
        if chars[i] == '/' && i + 1 < len && chars[i + 1] == '*' {
            let start = i;
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            } else {
                i = len;
            }
            let s = buf.iter_at_offset(start as i32);
            let e = buf.iter_at_offset(i as i32);
            buf.apply_tag_by_name("css_comment", &s, &e);
            continue;
        }

        // Strings "..." or '...'
        if chars[i] == '"' || chars[i] == '\'' {
            let q = chars[i];
            let start = i;
            i += 1;
            while i < len && chars[i] != q {
                if chars[i] == '\\' && i + 1 < len {
                    i += 1;
                }
                i += 1;
            }
            if i < len {
                i += 1;
            }
            let s = buf.iter_at_offset(start as i32);
            let e = buf.iter_at_offset(i as i32);
            buf.apply_tag_by_name("css_str", &s, &e);
            continue;
        }

        // Punctuations
        if chars[i] == '{' {
            let s = buf.iter_at_offset(i as i32);
            let e = buf.iter_at_offset((i + 1) as i32);
            buf.apply_tag_by_name("css_punct", &s, &e);
            in_block = true;
            in_value = false;
            i += 1;
            continue;
        }
        if chars[i] == '}' {
            let s = buf.iter_at_offset(i as i32);
            let e = buf.iter_at_offset((i + 1) as i32);
            buf.apply_tag_by_name("css_punct", &s, &e);
            in_block = false;
            in_value = false;
            i += 1;
            continue;
        }
        if chars[i] == ':' {
            let s = buf.iter_at_offset(i as i32);
            let e = buf.iter_at_offset((i + 1) as i32);
            buf.apply_tag_by_name("css_punct", &s, &e);
            if in_block {
                in_value = true;
            }
            i += 1;
            continue;
        }
        if chars[i] == ';' {
            let s = buf.iter_at_offset(i as i32);
            let e = buf.iter_at_offset((i + 1) as i32);
            buf.apply_tag_by_name("css_punct", &s, &e);
            in_value = false;
            i += 1;
            continue;
        }

        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }

        // Token
        let start = i;
        while i < len
            && !chars[i].is_whitespace()
            && chars[i] != '{'
            && chars[i] != '}'
            && chars[i] != ':'
            && chars[i] != ';'
            && chars[i] != '"'
            && chars[i] != '\''
            && !(chars[i] == '/' && i + 1 < len && chars[i + 1] == '*')
        {
            i += 1;
        }

        let s = buf.iter_at_offset(start as i32);
        let e = buf.iter_at_offset(i as i32);
        if !in_block {
            buf.apply_tag_by_name("css_selector", &s, &e);
        } else if !in_value {
            buf.apply_tag_by_name("css_prop", &s, &e);
        } else {
            let first = chars[start];
            if first.is_ascii_digit() || first == '#' || first == '.' {
                buf.apply_tag_by_name("css_num", &s, &e);
            } else {
                buf.apply_tag_by_name("css_val", &s, &e);
            }
        }
    }
}
