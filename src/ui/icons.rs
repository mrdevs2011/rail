//! Inline SVG icons drawn into GTK images.

use crate::*;

pub(crate) const GOOGLE_G_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24">
  <path fill="#4285F4" d="M23.745 12.27c0-.7-.06-1.4-.19-2.07H12v4.51h6.6c-.29 1.52-1.14 2.82-2.4 3.68v3.05h3.88c2.27-2.09 3.66-5.17 3.66-9.17z"/>
  <path fill="#34A853" d="M12 24c3.24 0 5.95-1.08 7.93-2.91l-3.88-3.05c-1.08.72-2.45 1.16-4.05 1.16-3.12 0-5.77-2.1-6.72-4.93H1.25v3.15C3.27 21.36 7.33 24 12 24z"/>
  <path fill="#FBBC05" d="M5.28 14.27c-.25-.72-.38-1.49-.38-2.27s.13-1.55.38-2.27V6.58H1.25C.45 8.17 0 9.99 0 12s.45 3.83 1.25 5.42l4.03-3.15z"/>
  <path fill="#EA4335" d="M12 4.75c1.77 0 3.35.61 4.6 1.8l3.42-3.42C17.95 1.19 15.24 0 12 0 7.33 0 3.27 2.64 1.25 6.58l4.03 3.15c.95-2.83 3.6-4.98 6.72-4.98z"/>
</svg>"##;

pub(crate) fn create_google_icon(size: i32) -> gtk::Image {
    let bytes = glib::Bytes::from_static(GOOGLE_G_SVG.as_bytes());
    match gdk::Texture::from_bytes(&bytes) {
        Ok(t) => {
            let img = gtk::Image::from_paintable(Some(&t));
            img.set_pixel_size(size);
            img
        }
        Err(_) => {
            let img = gtk::Image::from_icon_name("system-search-symbolic");
            img.set_pixel_size(size);
            img
        }
    }
}

/// Rail-style settings gear (SVG, not emoji / theme icon)
pub(crate) const GEAR_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="#FFFFFF">
  <path d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.07.62-.07.94s.02.64.07.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .43-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"/>
</svg>"##;

pub(crate) fn create_gear_icon(size: i32) -> gtk::Image {
    let bytes = glib::Bytes::from_static(GEAR_SVG.as_bytes());
    match gdk::Texture::from_bytes(&bytes) {
        Ok(t) => {
            let img = gtk::Image::from_paintable(Some(&t));
            img.set_pixel_size(size);
            img
        }
        Err(_) => {
            let img = gtk::Image::from_icon_name("preferences-system-symbolic");
            img.set_pixel_size(size);
            img
        }
    }
}

/// "Translate" glyph (A / 文) for the page context menu. Same geometry as the original
/// SVG Repo icon, with the stroke inlined as attributes (no <style>) so every renderer agrees.
pub(crate) const TRANSLATE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="#9a9996" stroke-width="1.92" stroke-miterlimit="10">
  <line x1="0.5" y1="3.35" x2="12" y2="3.35"/>
  <line x1="6.25" y1="0.48" x2="6.25" y2="3.35"/>
  <path d="M9.12,3.35c0,3.52-3.28,8.2-7.66,10.55"/>
  <path d="M4.51,7.37A16.4,16.4,0,0,0,11,13.9"/>
  <polyline points="12.96 22.52 16.79 11.98 17.75 11.98 21.58 22.52"/>
  <line x1="20.43" y1="18.69" x2="15.07" y2="18.69"/>
  <line x1="11.04" y1="22.52" x2="14.88" y2="22.52"/>
  <line x1="19.67" y1="22.52" x2="23.5" y2="22.52"/>
</svg>"##;

thread_local! {
    static TRANSLATE_ICON_READY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Serialized GIcon ("rail-translate") for GMenu items. The SVG is written once to
/// ~/.cache/rail/icons and that folder is added to the GTK icon search path (unthemed icon).
pub(crate) fn translate_icon_variant() -> Option<glib::Variant> {
    if !TRANSLATE_ICON_READY.with(|r| r.get()) {
        let dir = glib::user_cache_dir().join("rail").join("icons");
        std::fs::create_dir_all(&dir).ok()?;
        std::fs::write(dir.join("rail-translate.svg"), TRANSLATE_SVG).ok()?;
        let display = gdk::Display::default()?;
        gtk::IconTheme::for_display(&display).add_search_path(&dir);
        TRANSLATE_ICON_READY.with(|r| r.set(true));
    }
    gio::Icon::serialize(gio::ThemedIcon::new("rail-translate").upcast_ref::<gio::Icon>())
}
