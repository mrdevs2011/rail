//! Live shortcut labels in tooltips, badges and hints.

use crate::*;

// ---- UI'da ko'rsatiladigan shortcut yozuvlari ---------------------------------------------
// Tooltip, badge va hint'lardagi shortcut matnlari qotirilmaydi: Settings > Customize > Shortcuts
// dagi joriy qiymat ko'rsatiladi va u o'zgarganda barcha yozuvlar darhol yangilanadi.

thread_local! {
    /// Shortcut matni ko'rsatadigan vidjetlarni yangilovchi closure'lar (false qaytarsa vidjet o'lgan, olib tashlanadi).
    pub(crate) static SC_BOUND: RefCell<Vec<Box<dyn Fn() -> bool>>> = RefCell::new(Vec::new());
}

/// `id` shortcut'ining hozir amalda bo'lgan birinchi kombinatsiyasi. O'chirilgan yoki bo'sh bo'lsa "".
pub(crate) fn sc_display(id: &str) -> String {
    SHORTCUTS.with(|c| {
        let cfg = c.borrow();
        let Some(def) = SHORTCUT_DEFS.iter().find(|d| d.id == id) else {
            return String::new();
        };
        if !sc_enabled(&cfg, id) {
            return String::new();
        }
        sc_effective(def, &cfg)
            .into_iter()
            .find(|s| !s.is_empty())
            .unwrap_or_default()
    })
}

/// O'chirilgan shortcut belgisini matndan chiroyli olib tashlaydi: "(Alt+P)", " / F12", "F12 / ".
pub(crate) fn sc_strip_token(mut s: String, tok: &str) -> String {
    let paren = format!("({})", tok);
    if let Some(i) = s.find(&paren) {
        let head = s[..i].trim_end().to_string();
        let tail = s[i + paren.len()..].to_string();
        return format!("{}{}", head, tail);
    }
    for pat in [format!(" / {}", tok), format!("{} / ", tok)] {
        if s.contains(&pat) {
            return s.replacen(&pat, "", 1);
        }
    }
    s = s.replace(tok, "");
    s.trim().to_string()
}

/// `text` tarjima qilingan matn bo'lib, ichida standart kombinatsiyalar (masalan "Alt+Shift+V") bor.
/// Har bir `ids[i]` uchun uning standart kombinatsiyasi joriy kombinatsiyaga almashtiriladi.
/// Ikki bosqichli (belgi orqali): "Ctrl+T / Ctrl+L" da ikkalasi o'rin almashtirilsa ham to'qnashmaydi.
pub(crate) fn sc_apply(text: &str, ids: &[&str]) -> String {
    let tok = |i: usize| format!("\u{1}{}\u{2}", i);
    let mut out = text.to_string();
    for (i, id) in ids.iter().enumerate() {
        if let Some(def) = SHORTCUT_DEFS.iter().find(|d| d.id == *id) {
            out = out.replace(def.defaults[0], &tok(i));
        }
    }
    for (i, id) in ids.iter().enumerate() {
        let t = tok(i);
        let cur = sc_display(id);
        out = if cur.is_empty() {
            sc_strip_token(out, &t)
        } else {
            out.replace(&t, &cur)
        };
    }
    out
}

pub(crate) fn sc_bind(f: impl Fn() -> bool + 'static) {
    if !f() {
        return;
    }
    SC_BOUND.with(|b| {
        let mut v = b.borrow_mut();
        v.push(Box::new(f));
        // Qayta-qayta yaratiladigan vidjetlar (sidebar) ro'yxatni shishirmasin
        if v.len() > 128 {
            v.retain(|f| f());
        }
    });
}

/// Config o'zgarganda (Settings'da saqlash, import, ishga tushirish) hamma yozuvni yangilaydi.
pub(crate) fn sc_refresh_bound() {
    SC_BOUND.with(|b| b.borrow_mut().retain(|f| f()));
}

/// Label matni: ingliz kalit -> tarjima -> standart shortcutlar joriylari bilan almashtiriladi.
pub(crate) fn sc_bind_text(lbl: &gtk::Label, key: &'static str, ids: &'static [&'static str]) {
    let weak = lbl.downgrade();
    sc_bind(move || match weak.upgrade() {
        Some(l) => {
            l.set_text(&sc_apply(tr(key), ids));
            true
        }
        None => false,
    });
}

/// Faqat shortcut ko'rsatadigan chip/badge: shortcut o'chirilgan bo'lsa yashiriladi.
pub(crate) fn sc_bind_chip(lbl: &gtk::Label, id: &'static str) {
    let weak = lbl.downgrade();
    sc_bind(move || match weak.upgrade() {
        Some(l) => {
            let t = sc_display(id);
            l.set_visible(!t.is_empty());
            l.set_text(&t);
            true
        }
        None => false,
    });
}

pub(crate) fn sc_bind_tooltip<W: gtk::prelude::IsA<gtk::Widget>>(
    w: &W,
    key: &'static str,
    ids: &'static [&'static str],
) {
    let weak = w.upcast_ref::<gtk::Widget>().downgrade();
    sc_bind(move || match weak.upgrade() {
        Some(w) => {
            w.set_tooltip_text(Some(&sc_apply(tr(key), ids)));
            true
        }
        None => false,
    });
}

/// Website shortcuts always have precedence: Rail handles a registered shortcut only
/// when the focused page did not consume the key event.
pub(crate) fn shortcut_propagation_phase() -> gtk::PropagationPhase {
    gtk::PropagationPhase::Bubble
}

pub(crate) fn apply_shortcut_priority() {
    KEY_CTL.with(|k| {
        if let Some(ctl) = k.borrow().as_ref() {
            ctl.set_propagation_phase(shortcut_propagation_phase());
        }
    });
}
