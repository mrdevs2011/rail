//! Icons loaded from `icons/svg/` and `icons/png/` — never inlined in source.

use crate::*;

/// SVG file contents from the repo `icons/svg/` tree (compile-time include).
fn svg_data(name: &str) -> Option<&'static [u8]> {
    Some(match name {
        "google" => include_bytes!("../../icons/svg/google.svg").as_slice(),
        "bing" => include_bytes!("../../icons/svg/bing.svg").as_slice(),
        "duckduckgo" => include_bytes!("../../icons/svg/duckduckgo.svg").as_slice(),
        "yandex" => include_bytes!("../../icons/svg/yandex.svg").as_slice(),
        "yahoo" => include_bytes!("../../icons/svg/yahoo.svg").as_slice(),
        "brave" => include_bytes!("../../icons/svg/brave.svg").as_slice(),
        "baidu" => include_bytes!("../../icons/svg/baidu.svg").as_slice(),
        "ecosia" => include_bytes!("../../icons/svg/ecosia.svg").as_slice(),
        "startpage" => include_bytes!("../../icons/svg/startpage.svg").as_slice(),
        "qwant" => include_bytes!("../../icons/svg/qwant.svg").as_slice(),
        "perplexity" => include_bytes!("../../icons/svg/perplexity.svg").as_slice(),
        "searxng" => include_bytes!("../../icons/svg/searxng.svg").as_slice(),
        "kagi" => include_bytes!("../../icons/svg/kagi.svg").as_slice(),
        "naver" => include_bytes!("../../icons/svg/naver.svg").as_slice(),
        "ask" => include_bytes!("../../icons/svg/ask.svg").as_slice(),
        "gear" => include_bytes!("../../icons/svg/gear.svg").as_slice(),
        "translate" => include_bytes!("../../icons/svg/translate.svg").as_slice(),
        "user" => include_bytes!("../../icons/svg/user.svg").as_slice(),
        "auto-lang" => include_bytes!("../../icons/svg/auto-lang.svg").as_slice(),
        "back" => include_bytes!("../../icons/svg/back.svg").as_slice(),
        "forward" => include_bytes!("../../icons/svg/forward.svg").as_slice(),
        "reload" => include_bytes!("../../icons/svg/reload.svg").as_slice(),
        "qr" => include_bytes!("../../icons/svg/qr.svg").as_slice(),
        "translate-menu" => include_bytes!("../../icons/svg/translate-menu.svg").as_slice(),
        "source" => include_bytes!("../../icons/svg/source.svg").as_slice(),
        "inspect" => include_bytes!("../../icons/svg/inspect.svg").as_slice(),
        "screenshot" => include_bytes!("../../icons/svg/screenshot.svg").as_slice(),
        "rail" => include_bytes!("../../icons/svg/rail.svg").as_slice(),
        _ => return None,
    })
}

/// Optional PNG from `icons/png/{name}.png` (runtime, next to data dir or CWD).
fn png_path(name: &str) -> Option<std::path::PathBuf> {
    let candidates = [
        PathBuf::from("icons/png").join(format!("{name}.png")),
        rail_data_dir().join("icons/png").join(format!("{name}.png")),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

/// Load a named icon: prefers `icons/png/{name}.png` if present, else `icons/svg/{name}.svg`.
pub(crate) fn load_icon(name: &str, size: i32) -> gtk::Image {
    if let Some(path) = png_path(name) {
        if let Ok(tex) = gdk::Texture::from_filename(&path) {
            let img = gtk::Image::from_paintable(Some(&tex));
            img.set_pixel_size(size);
            return img;
        }
    }
    if let Some(bytes) = svg_data(name) {
        if let Ok(tex) = gdk::Texture::from_bytes(&glib::Bytes::from_static(bytes)) {
            let img = gtk::Image::from_paintable(Some(&tex));
            img.set_pixel_size(size);
            return img;
        }
    }
    let img = gtk::Image::from_icon_name("image-missing");
    img.set_pixel_size(size);
    img
}

/// Raw SVG bytes for a known name (e.g. flag auto-detect, translate theme install).
pub(crate) fn svg_bytes(name: &str) -> Option<&'static [u8]> {
    svg_data(name)
}

pub(crate) fn canonical_search_engine(engine: &str) -> &'static str {
    match engine.to_lowercase().as_str() {
        "google" => "google",
        "bing" => "bing",
        "duckduckgo" | "ddg" => "duckduckgo",
        "yandex" => "yandex",
        "yahoo" => "yahoo",
        "brave" => "brave",
        "baidu" => "baidu",
        "ecosia" => "ecosia",
        "startpage" => "startpage",
        "qwant" => "qwant",
        "perplexity" => "perplexity",
        "searxng" | "searx" => "searxng",
        "kagi" => "kagi",
        "naver" => "naver",
        "ask" => "ask",
        _ => "google",
    }
}

pub(crate) fn search_engine_display_name(engine: &str) -> &'static str {
    match engine.to_lowercase().as_str() {
        "google" => "Google",
        "bing" => "Microsoft Bing",
        "duckduckgo" | "ddg" => "DuckDuckGo",
        "yandex" => "Yandex",
        "yahoo" => "Yahoo!",
        "brave" => "Brave Search",
        "baidu" => "Baidu",
        "ecosia" => "Ecosia",
        "startpage" => "Startpage",
        "qwant" => "Qwant",
        "perplexity" => "Perplexity",
        "searxng" | "searx" => "SearXNG",
        "kagi" => "Kagi",
        "naver" => "Naver",
        "ask" => "Ask.com",
        _ => "Google",
    }
}

pub(crate) fn search_engine_svg_bytes(engine: &str) -> &'static [u8] {
    svg_data(canonical_search_engine(engine)).unwrap_or(b"")
}

pub(crate) fn load_search_engine_icon(engine: &str, size: i32) -> gtk::Image {
    load_icon(canonical_search_engine(engine), size)
}

pub(crate) fn create_google_icon(size: i32) -> gtk::Image {
    load_icon("google", size)
}

pub(crate) fn create_gear_icon(size: i32) -> gtk::Image {
    load_icon("gear", size)
}

pub(crate) fn create_user_icon(size: i32) -> gtk::Image {
    load_icon("user", size)
}

pub(crate) fn create_rail_icon(size: i32) -> gtk::Image {
    load_icon("rail", size)
}

thread_local! {
    static TRANSLATE_ICON_READY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Serialized GIcon ("rail-translate") for GMenu items. Copies `icons/svg/translate.svg`
/// into ~/.cache/rail/icons once so GTK can resolve the themed name.
pub(crate) fn translate_icon_variant() -> Option<glib::Variant> {
    if !TRANSLATE_ICON_READY.with(|r| r.get()) {
        let dir = glib::user_cache_dir().join("rail").join("icons");
        std::fs::create_dir_all(&dir).ok()?;
        let bytes = svg_bytes("translate")?;
        std::fs::write(dir.join("rail-translate.svg"), bytes).ok()?;
        let display = gdk::Display::default()?;
        gtk::IconTheme::for_display(&display).add_search_path(&dir);
        TRANSLATE_ICON_READY.with(|r| r.set(true));
    }
    gio::Icon::serialize(gio::ThemedIcon::new("rail-translate").upcast_ref::<gio::Icon>())
}
