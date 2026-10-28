//! Settings tab: header, tab buttons, search engines, data and profile actions.

use crate::*;

pub(crate) fn wire_settings(app: &Rc<App>, add_prof_btn: gtk::Button, btn_clear_cache: gtk::Button, btn_clear_hist: gtk::Button, btn_copy_sync_url: gtk::Button, btn_export_local: gtk::Button, btn_import_local: gtk::Button, btn_sync_now: gtk::Button, btn_sync_restore: gtk::Button, engine_buttons: Vec<(String, gtk::Button)>, layout: StripLayout, new_profile_entry: gtk::Entry, settings_close_btn: gtk::Button, settings_update_chip: gtk::Button, sync_restore_entry: gtk::Entry, sync_url_result_entry: gtk::Entry, tab_buttons_vec: Vec<(SettingsTab, gtk::Button)>, win: gtk::ApplicationWindow) {
    let a = app.clone();
    settings_close_btn.connect_clicked(move |_| {
        // X = close the settings tab (same as Ctrl+W), not just hide chrome
        if let Some(v) = a.focused_view() {
            let is_settings = v
                .uri()
                .map(|u| u.starts_with("rail://settings"))
                .unwrap_or(false)
                || a.saved_uri_of(&v).starts_with("rail://settings");
            if is_settings {
                a.close_view(&v);
                return;
            }
        }
        a.hide_settings_modal();
    });

    {
        let a = app.clone();
        settings_update_chip.connect_clicked(move |_| {
            a.open_settings_tab(SettingsTab::Update);
        });
    }

    // Populate header "New version" chip from GitHub (same source as Updates page)
    {
        let chip = settings_update_chip.clone();
        glib::spawn_future_local(async move {
            let res = gio::spawn_blocking(update::fetch_update).await;
            if let Ok(Some(inf)) = res {
                chip.set_label(&trf("New: {}", &[&inf.latest_tag]));
                chip.set_visible(true);
            }
        });
    }

    for (tab_type, btn) in tab_buttons_vec.iter() {
        let a = app.clone();
        let t = *tab_type;
        btn.connect_clicked(move |_| a.open_settings_tab(t));
    }

    for (engine_id, btn) in engine_buttons.iter() {
        let a = app.clone();
        let id_str = engine_id.clone();
        let engine_btns_clone = engine_buttons.clone();
        btn.connect_clicked(move |_| {
            *a.search_engine.borrow_mut() = id_str.clone();
            for (eid, b) in engine_btns_clone.iter() {
                if eid == &id_str {
                    b.add_css_class("active");
                } else {
                    b.remove_css_class("active");
                }
            }
            a.sync_search_engine_ui();
            a.save_session_immediate();
            let mut s = load_app_settings();
            s.search_engine = id_str.clone();
            save_app_settings(&s);
        });
    }

    let a = app.clone();
    btn_clear_hist.connect_clicked(move |_| a.clear_history_action());

    let a = app.clone();
    btn_clear_cache.connect_clicked(move |_| a.clear_cache_action());

    let a = app.clone();

    let a = app.clone();
    btn_sync_now.connect_clicked(move |_| a.prompt_and_execute_sync());

    let a = app.clone();
    btn_sync_restore.connect_clicked(move |_| a.prompt_and_execute_restore());

    let a = app.clone();
    sync_restore_entry.connect_activate(move |_| a.prompt_and_execute_restore());

    let a = app.clone();
    let entry_res = sync_url_result_entry.clone();
    btn_copy_sync_url.connect_clicked(move |btn| {
        let text = entry_res.text().to_string();
        if !text.is_empty() {
            a.win.clipboard().set_text(&text);
            btn.set_label(tr("Copied!"));
            let b = btn.clone();
            glib::timeout_add_local_once(Duration::from_millis(1500), move || {
                b.set_label(tr("Copy Link"));
            });
        }
    });

    let a = app.clone();
    let win_weak = win.downgrade();
    btn_export_local.connect_clicked(move |_| {
        let Some(win) = win_weak.upgrade() else { return };
        let timestamp = glib::DateTime::now_local()
            .ok()
            .and_then(|d| d.format("%Y%m%d_%H%M%S").ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "backup".to_string());
        let default_name = format!("rail_backup_{}.tar.gz", timestamp);
        let dialog = gtk::FileDialog::builder()
            .title(tr("Save Rail Backup Archive (.tar.gz)"))
            .initial_name(&default_name)
            .build();
        let a = a.clone();
        dialog.save(Some(&win), gio::Cancellable::NONE, move |res| {
            if let Ok(file) = res {
                if let Some(path) = file.path() {
                    a.data_export_entry.set_text(&path.to_string_lossy());
                    a.execute_export_action();
                }
            }
        });
    });

    let a = app.clone();
    let win_weak = win.downgrade();
    btn_import_local.connect_clicked(move |_| {
        let Some(win) = win_weak.upgrade() else { return };
        let dialog = gtk::FileDialog::builder()
            .title(tr("Select Rail Backup Archive (.tar.gz)"))
            .build();
        let a = a.clone();
        dialog.open(Some(&win), gio::Cancellable::NONE, move |res| {
            if let Ok(file) = res {
                if let Some(path) = file.path() {
                    a.data_import_entry.set_text(&path.to_string_lossy());
                    a.execute_import_action();
                }
            }
        });
    });

    for (i, btn) in app.color_buttons.borrow().iter().enumerate() {
        let a = app.clone();
        btn.connect_clicked(move |_| a.select_new_profile_color(i));
    }

    let a = app.clone();
    add_prof_btn.connect_clicked(move |_| a.create_new_profile_action());
    let a = app.clone();
    new_profile_entry.connect_activate(move |_| a.create_new_profile_action());

    let a = app.clone();
    new_profile_entry.connect_changed(move |_| {
        a.update_create_profile_btn_state();
    });

    {
        // Off-screen columns are unmapped by the strip layout (WebKit then throttles them). Grab the last
        // frame just before a live tab is hidden (so suspend can still freeze it) and drop it when shown.
        let weak = Rc::downgrade(&app);
        layout.connect_visibility(move |child, visible| {
            let Some(a) = weak.upgrade() else { return };
            let Ok(rows) = a.rows.try_borrow() else { return };
            for row in rows.iter() {
                for tab in row.tabs.iter() {
                    if tab.container.upcast_ref::<gtk::Widget>() != child {
                        continue;
                    }
                    if visible {
                        if tab.suspended.get() {
                            // Overview / slide-in: show last frame if we have one (not a blank panel).
                            if tab.freeze.paintable().is_some() {
                                tab.freeze.set_visible(true);
                            }
                        } else {
                            // Live tab remapped: keep last frame over WebKit until it paints again
                            // (clear_tab_freeze runs when is_loading goes false).
                            if tab.freeze.paintable().is_some() {
                                tab.freeze.set_visible(true);
                            } else {
                                tab.freeze.set_visible(false);
                            }
                            tab.view.queue_draw();
                            // If not loading, drop the cover on the next idle so we do not stick.
                            if !tab.view.is_loading() {
                                let view = tab.view.clone();
                                let app2 = a.clone();
                                glib::idle_add_local_once(move || {
                                    app2.clear_tab_freeze(&view);
                                });
                            }
                        }
                    } else if !tab.suspended.get() {
                        // About to unmap: durable last frame for suspend / overview (never live WidgetPaintable).
                        App::store_tab_frame(tab);
                    }
                    return;
                }
            }
        });
    }

    {
        let weak = Rc::downgrade(&app);
        app.win.connect_fullscreened_notify(move |_| {
            if let Some(a) = weak.upgrade() {
                a.apply_fullscreen_mode();
            }
        });
    }

    {
        let a = app.clone();
        win.connect_default_width_notify(move |_| {
            if a.settings_modal_open() {
                a.sync_settings_responsive();
            }
        });
    }
}
