//! Profile list and profile creation.

use crate::*;

impl App {
    pub(crate) fn rebuild_profiles_modal_list(self: &Rc<Self>) {
        while let Some(r) = self.profiles_list_box.row_at_index(0) {
            self.profiles_list_box.remove(&r);
        }

        let (profiles, active_id, can_delete) = {
            let mgr = self.profiles.borrow();
            (mgr.profiles().to_vec(), mgr.active_id().to_string(), mgr.profiles().len() > 1)
        };
        let total_profiles = profiles.len();
        let rows_vec: Rc<RefCell<Vec<gtk::Box>>> = Rc::new(RefCell::new(Vec::new()));
        let drag_provider = Rc::new(gtk::CssProvider::new());
        if let Some(disp) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &disp,
                &*drag_provider,
                gtk::STYLE_PROVIDER_PRIORITY_USER,
            );
        }

        for (idx, p) in profiles.into_iter().enumerate() {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            row.add_css_class("profile-card");
            if p.id == active_id {
                row.add_css_class("active-profile");
            }
            rows_vec.borrow_mut().push(row.clone());

            let drag_handle = gtk::Label::new(Some("⋮⋮"));
            drag_handle.add_css_class("profile-drag-handle");
            drag_handle.set_cursor_from_name(Some("grab"));
            drag_handle.set_valign(gtk::Align::Center);
            drag_handle.set_tooltip_text(Some(tr("Drag up or down to reorder profiles")));

            if total_profiles > 1 {
                let drag = gtk::GestureDrag::new();
                drag.set_button(gdk::BUTTON_PRIMARY);

                let row_drag = row.clone();
                let handle_drag = drag_handle.clone();
                let drag_provider_begin = drag_provider.clone();
                drag.connect_drag_begin(move |_gesture, _start_x, _start_y| {
                    row_drag.add_css_class("dragging");
                    row_drag.set_cursor_from_name(Some("grabbing"));
                    handle_drag.set_cursor_from_name(Some("grabbing"));
                    drag_provider_begin.load_from_string(".profile-card.dragging { z-index: 1000 !important; }");
                });

                let rows_vec_update = rows_vec.clone();
                let drag_provider_update = drag_provider.clone();
                let from_idx = idx;
                drag.connect_drag_update(move |_gesture, _offset_x, offset_y| {
                    drag_provider_update.load_from_string(&format!(
                        ".profile-card.dragging {{ transform: translateY({:.1}px) !important; z-index: 1000 !important; }}",
                        offset_y
                    ));

                    let stride = 60.0f64;
                    let delta = (offset_y / stride).round() as isize;
                    let target_idx = ((from_idx as isize) + delta).clamp(0, (total_profiles - 1) as isize) as usize;

                    let rows_ref = rows_vec_update.borrow();
                    for (i, r_box) in rows_ref.iter().enumerate() {
                        if i == from_idx {
                            continue;
                        }
                        if from_idx < target_idx {
                            if i > from_idx && i <= target_idx {
                                r_box.add_css_class("shift-up");
                                r_box.remove_css_class("shift-down");
                            } else {
                                r_box.remove_css_class("shift-up");
                                r_box.remove_css_class("shift-down");
                            }
                        } else if from_idx > target_idx {
                            if i >= target_idx && i < from_idx {
                                r_box.add_css_class("shift-down");
                                r_box.remove_css_class("shift-up");
                            } else {
                                r_box.remove_css_class("shift-up");
                                r_box.remove_css_class("shift-down");
                            }
                        } else {
                            r_box.remove_css_class("shift-up");
                            r_box.remove_css_class("shift-down");
                        }
                    }
                });

                let row_end = row.clone();
                let handle_end = drag_handle.clone();
                let app_end = self.clone();
                let from_idx = idx;
                let rows_vec_end = rows_vec.clone();
                let drag_provider_end = drag_provider.clone();
                drag.connect_drag_end(move |_gesture, _offset_x, offset_y| {
                    drag_provider_end.load_from_string("");
                    row_end.remove_css_class("dragging");
                    row_end.set_cursor_from_name(Some("default"));
                    handle_end.set_cursor_from_name(Some("grab"));

                    let rows_ref = rows_vec_end.borrow();
                    for r_box in rows_ref.iter() {
                        r_box.remove_css_class("shift-up");
                        r_box.remove_css_class("shift-down");
                    }
                    drop(rows_ref);

                    let stride = 60.0f64;
                    let delta = (offset_y / stride).round() as isize;
                    let target_idx = ((from_idx as isize) + delta).clamp(0, (total_profiles - 1) as isize) as usize;

                    if target_idx != from_idx {
                        if app_end.profiles.borrow_mut().reorder_profile(from_idx, target_idx) {
                            let a = app_end.clone();
                            glib::idle_add_local_once(move || {
                                a.rebuild_profiles_modal_list();
                                a.rebuild_profile_popover();
                                a.refresh_sidebar();
                            });
                        }
                    }
                });

                drag_handle.add_controller(drag);
            }

            let avatar = gtk::Label::new(None);
            let fg = p.text_color();
            avatar.set_markup(&format!(
                "<span foreground='{}' font_weight='800'>{}</span>",
                fg,
                glib::markup_escape_text(&p.initial_letter())
            ));
            avatar.add_css_class("profile-avatar-circle");
            avatar.add_css_class(&format!("prof-bg-{}", p.color_idx));
            avatar.set_valign(gtk::Align::Center);
            avatar.set_halign(gtk::Align::Center);
            avatar.set_width_request(32);
            avatar.set_height_request(32);

            let info_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
            info_box.set_hexpand(true);
            info_box.set_valign(gtk::Align::Center);

            let name_lbl = gtk::Label::new(Some(&p.display_name()));
            name_lbl.add_css_class("profile-card-name");
            name_lbl.set_xalign(0.0);
            name_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
            name_lbl.set_wrap(false);

            let sub_text = trf("@{} - isolated storage", &[&p.id]);
            let sub_lbl = gtk::Label::new(Some(&sub_text));
            sub_lbl.add_css_class("profile-card-sub");
            sub_lbl.set_xalign(0.0);
            sub_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
            sub_lbl.set_wrap(false);

            info_box.append(&name_lbl);
            info_box.append(&sub_lbl);

            let actions_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            actions_box.set_valign(gtk::Align::Center);

            if p.id == active_id {
                let active_badge = gtk::Label::new(Some(tr("Active")));
                active_badge.add_css_class("profile-badge-active");
                actions_box.append(&active_badge);
            } else {
                let set_act_btn = gtk::Button::with_label(tr("Make Active"));
                set_act_btn.add_css_class("btn-make-active");
                let app = self.clone();
                let pid = p.id.clone();
                set_act_btn.connect_clicked(move |_| {
                    app.profiles.borrow_mut().set_active(&pid);
                    let a = app.clone();
                    glib::idle_add_local_once(move || {
                        a.rebuild_profiles_modal_list();
                        a.rebuild_profile_popover();
                        a.refresh_sidebar();
                    });
                });
                actions_box.append(&set_act_btn);
            }

            if can_delete && p.id != "default" {
                let del_btn = gtk::Button::from_icon_name("window-close-symbolic");
                del_btn.add_css_class("btn-del-profile");
                del_btn.set_tooltip_text(Some(tr("Delete Profile")));
                let app = self.clone();
                let pid = p.id.clone();
                del_btn.connect_clicked(move |_| {
                    app.profiles.borrow_mut().delete_profile(&pid);
                    {
                        let mut rows = app.rows.borrow_mut();
                        for row in rows.iter_mut() {
                            for tab in row.tabs.iter_mut() {
                                if tab.profile_id == pid {
                                    tab.profile_id = "default".to_string();
                                }
                            }
                        }
                    }
                    let a = app.clone();
                    glib::idle_add_local_once(move || {
                        a.rebuild_profiles_modal_list();
                        a.rebuild_profile_popover();
                        a.refresh_sidebar();
                    });
                });
                actions_box.append(&del_btn);
            }

            row.append(&drag_handle);
            row.append(&avatar);
            row.append(&info_box);
            row.append(&actions_box);

            self.profiles_list_box.append(&row);
        }
    }

    pub(crate) fn select_new_profile_color(&self, idx: usize) {
        self.new_profile_color_idx.set(idx);
        let btns = self.color_buttons.borrow();
        for (i, b) in btns.iter().enumerate() {
            if i == idx {
                b.add_css_class("selected");
            } else {
                b.remove_css_class("selected");
            }
        }
    }

    pub(crate) fn update_create_profile_btn_state(self: &Rc<Self>) {
        let text = self.new_profile_entry.text().to_string();
        let clean = text.trim();
        if clean.is_empty() {
            self.new_profile_error_lbl.set_visible(false);
            self.new_profile_entry.remove_css_class("error");
            self.add_prof_btn.set_sensitive(false);
            self.add_prof_btn.remove_css_class("active-ready");
        } else if self.profiles.borrow().profile_exists(clean) {
            self.new_profile_error_lbl.set_text(tr("Profile already exists"));
            self.new_profile_error_lbl.set_visible(true);
            self.new_profile_entry.add_css_class("error");
            self.add_prof_btn.set_sensitive(false);
            self.add_prof_btn.remove_css_class("active-ready");
        } else {
            self.new_profile_error_lbl.set_visible(false);
            self.new_profile_entry.remove_css_class("error");
            self.add_prof_btn.set_sensitive(true);
            self.add_prof_btn.add_css_class("active-ready");
        }
    }

    pub(crate) fn create_new_profile_action(self: &Rc<Self>) {
        let name = self.new_profile_entry.text().to_string();
        let clean = name.trim().to_string();
        if clean.is_empty() {
            self.update_create_profile_btn_state();
            return;
        }

        let color_idx = self.new_profile_color_idx.get();
        let create_res = self.profiles.borrow_mut().create_profile(&clean, color_idx);
        match create_res {
            Ok(_new_p) => {
                self.new_profile_entry.set_text("");
                self.update_create_profile_btn_state();
                self.rebuild_profiles_modal_list();
                self.rebuild_profile_popover();
                self.refresh_sidebar();

                let scroll = self.profiles_modal_scroll.clone();
                glib::idle_add_local_once(move || {
                    let adj = scroll.vadjustment();
                    adj.set_value(adj.upper());
                });
            }
            Err(err) => {
                self.new_profile_error_lbl.set_text(tr(err));
                self.new_profile_error_lbl.set_visible(true);
                self.new_profile_entry.add_css_class("error");
                self.add_prof_btn.set_sensitive(false);
                self.add_prof_btn.remove_css_class("active-ready");
            }
        }
    }
}
