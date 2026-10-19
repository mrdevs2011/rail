//! Page translation: Google Translate proxy URLs (`<host>.translate.goog`).

use crate::*;

/// ISO 639-1 languages for Settings > Page Translation: (code, Uzbek name). "auto" is source-only.
pub(crate) const SUPPORTED_LANGUAGES: &[(&str, &str)] = &[
    ("auto", "Avto aniqlash (Auto Detect)"),
    ("uz", "O‘zbek"),
    ("en", "Ingliz (English)"),
    ("ru", "Rus (Русский)"),
    ("ab", "Abxaz"),
    ("aa", "Afar"),
    ("af", "Afrikaans"),
    ("sq", "Alban"),
    ("am", "Amxar"),
    ("ar", "Arab"),
    ("hy", "Arman"),
    ("as", "Assom"),
    ("av", "Avar"),
    ("ay", "Aymara"),
    ("az", "Ozarbayjon"),
    ("bm", "Bambara"),
    ("ba", "Boshqird"),
    ("eu", "Bask"),
    ("be", "Belarus"),
    ("bn", "Bengal"),
    ("bs", "Bosniya"),
    ("br", "Breton"),
    ("bg", "Bolgar"),
    ("my", "Myanma (Birma)"),
    ("ca", "Katalan"),
    ("ch", "Chamorro"),
    ("ce", "Chechen"),
    ("ny", "Chicheva"),
    ("zh", "Xitoy"),
    ("cv", "Chuvash"),
    ("co", "Korsika"),
    ("hr", "Xorvat"),
    ("cs", "Chex"),
    ("da", "Dan"),
    ("dv", "Divexi"),
    ("nl", "Golland"),
    ("dz", "Dzongka"),
    ("eo", "Esperanto"),
    ("et", "Eston"),
    ("ee", "Eve"),
    ("fo", "Farer"),
    ("fj", "Fiji"),
    ("fi", "Fin"),
    ("fr", "Fransuz"),
    ("fy", "Friz"),
    ("ff", "Fulani"),
    ("gl", "Galisiy"),
    ("ka", "Gruzin"),
    ("de", "Nemis"),
    ("el", "Grek"),
    ("gn", "Guarani"),
    ("gu", "Gujarot"),
    ("ht", "Gaiti-kreol"),
    ("ha", "Xausa"),
    ("he", "Ivrit"),
    ("hi", "Hind"),
    ("hu", "Venger"),
    ("is", "Island"),
    ("ig", "Igbo"),
    ("id", "Indonez"),
    ("ga", "Irland"),
    ("it", "Italyan"),
    ("ja", "Yapon"),
    ("jv", "Yava"),
    ("kn", "Kannada"),
    ("kr", "Kanuri"),
    ("ks", "Kashmiri"),
    ("kk", "Qozoq"),
    ("km", "Kxmer"),
    ("rw", "Kinyaruanda"),
    ("ky", "Qirg‘iz"),
    ("kv", "Komi"),
    ("kg", "Kikongo"),
    ("ko", "Koreys"),
    ("ku", "Kurd"),
    ("lo", "Laos"),
    ("la", "Lotin"),
    ("lv", "Latish"),
    ("li", "Limburg"),
    ("ln", "Lingala"),
    ("lt", "Litva"),
    ("lb", "Luksemburg"),
    ("mk", "Makedon"),
    ("mg", "Malagasi"),
    ("ms", "Malay"),
    ("ml", "Malayalam"),
    ("mt", "Maltiy"),
    ("gv", "Menks"),
    ("mi", "Maori"),
    ("mr", "Maratxi"),
    ("mn", "Mongol"),
    ("ne", "Nepal"),
    ("no", "Norveg"),
    ("or", "Odiya"),
    ("om", "Oromo"),
    ("os", "Osetin"),
    ("ps", "Pushtu"),
    ("fa", "Fors"),
    ("pl", "Polyak"),
    ("pt", "Portugal"),
    ("pa", "Panjob"),
    ("qu", "Kechua"),
    ("ro", "Rumin"),
    ("rn", "Rundi"),
    ("sm", "Samoa"),
    ("sg", "Sango"),
    ("sa", "Sanskrit"),
    ("sr", "Serb"),
    ("sn", "Shona"),
    ("sd", "Sindhi"),
    ("si", "Singal"),
    ("sk", "Slovak"),
    ("sl", "Sloven"),
    ("so", "Somali"),
    ("st", "Sesoto"),
    ("es", "Ispan"),
    ("su", "Sundan"),
    ("sw", "Suaxili"),
    ("ss", "Svati"),
    ("sv", "Shved"),
    ("tl", "Tagal"),
    ("tg", "Tojik"),
    ("ta", "Tamil"),
    ("tt", "Tatar"),
    ("te", "Telugu"),
    ("th", "Tay"),
    ("bo", "Tibet"),
    ("ti", "Tigrinya"),
    ("to", "Tonga"),
    ("ts", "Tsonga"),
    ("tn", "Tsvana"),
    ("tr", "Turk"),
    ("tk", "Turkman"),
    ("tw", "Tvi"),
    ("ug", "Uyg‘ur"),
    ("uk", "Ukrain"),
    ("ur", "Urdu"),
    ("ve", "Venda"),
    ("vi", "Vyetnam"),
    ("cy", "Valliy"),
    ("wo", "Volof"),
    ("xh", "Kxosa"),
    ("yi", "Yidish"),
    ("yo", "Yoruba"),
    ("zu", "Zulu"),
];

/// JS run in the page for source-language auto-detection: `<html lang>`, then `og:locale`.
pub(crate) const DETECT_LANG_JS: &str = "(function(){try{\
var l=document.documentElement.getAttribute('lang');\
if(l&&l.trim())return l.trim();\
var m=document.querySelector('meta[property=\"og:locale\"]');\
if(m&&m.content&&m.content.trim())return m.content.trim();\
}catch(e){}return '';})()";

/// Language code safe to put into a URL (`[A-Za-z0-9-]`, up to 8 chars), else `default`.
pub(crate) fn clean_lang_code(code: &str, default: &str) -> String {
    let c = code.trim();
    if !c.is_empty() && c.len() <= 8 && c.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-') {
        c.to_string()
    } else {
        default.to_string()
    }
}

/// Raw `lang` / `og:locale` value ("en-US", "en_US", "") -> Google code ("en") or "auto".
pub(crate) fn normalize_detected_lang(raw: &str) -> String {
    let t = raw.trim().to_lowercase().replace('_', "-");
    let first = t.split('-').next().unwrap_or("");
    if first.len() < 2 || first.len() > 3 || !first.chars().all(|c| c.is_ascii_lowercase()) {
        return "auto".to_string();
    }
    match first {
        "zh" => {
            if t.contains("tw") || t.contains("hk") || t.contains("hant") {
                "zh-TW".to_string()
            } else {
                "zh-CN".to_string()
            }
        }
        "nb" | "nn" => "no".to_string(),
        other => other.to_string(),
    }
}

/// `my-site.co.uk` -> `my--site-co-uk.translate.goog` (hyphen doubles first, then dots become hyphens).
pub(crate) fn translate_host(host: &str) -> String {
    format!("{}.translate.goog", host.replace('-', "--").replace('.', "-"))
}

/// Reverse of `translate_host` (without the `.translate.goog` suffix check done by the caller).
fn untranslate_host(translated: &str) -> Option<String> {
    let base = translated.strip_suffix(".translate.goog")?;
    let mut out = String::with_capacity(base.len());
    let mut it = base.chars().peekable();
    while let Some(c) = it.next() {
        if c == '-' {
            if it.peek() == Some(&'-') {
                it.next();
                out.push('-');
            } else {
                out.push('.');
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

fn rebuild_url(host: &str, u: &glib::Uri, extra: &[(&str, &str)]) -> String {
    let path = u.path();
    let mut out = format!("https://{}{}", host, if path.is_empty() { "/" } else { path.as_str() });
    let mut pairs: Vec<String> = u
        .query()
        .map(|q| q.split('&').filter(|p| !p.is_empty() && !p.starts_with("_x_tr_")).map(|p| p.to_string()).collect())
        .unwrap_or_default();
    for (k, v) in extra {
        pairs.push(format!("{}={}", k, v));
    }
    if !pairs.is_empty() {
        out.push('?');
        out.push_str(&pairs.join("&"));
    }
    if let Some(f) = u.fragment() {
        if !f.is_empty() {
            out.push('#');
            out.push_str(&f);
        }
    }
    out
}

/// Page already served through Google Translate.
pub(crate) fn is_translated_uri(uri: &str) -> bool {
    glib::Uri::parse(uri, glib::UriFlags::NONE)
        .ok()
        .and_then(|u| u.host())
        .map(|h| h.ends_with(".translate.goog"))
        .unwrap_or(false)
}

/// Settings code -> code Google Translate expects ("zh" alone is ambiguous there).
fn google_lang_code(code: &str) -> String {
    match code {
        "zh" => "zh-CN".to_string(),
        other => other.to_string(),
    }
}

/// Google Translate URL for `current`, or None when the page cannot be translated
/// (non-http(s), localhost / IP / custom port, already translated).
pub(crate) fn translate_url(current: &str, source: &str, target: &str) -> Option<String> {
    let u = glib::Uri::parse(current, glib::UriFlags::NONE).ok()?;
    if u.scheme() != "http" && u.scheme() != "https" {
        return None;
    }
    let host = u.host()?.to_string();
    if host.is_empty() || !host.contains('.') || host.contains(':') || host.ends_with(".translate.goog") || host.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }
    let port = u.port();
    if port != -1 && port != 80 && port != 443 {
        return None;
    }
    let sl = google_lang_code(&clean_lang_code(source, "auto"));
    let tl = google_lang_code(&clean_lang_code(target, "uz"));
    Some(rebuild_url(&translate_host(&host), &u, &[("_x_tr_sl", &sl), ("_x_tr_tl", &tl), ("_x_tr_hl", &tl)]))
}

/// Original page URL for a `*.translate.goog` address (None if `current` is not one).
pub(crate) fn untranslate_url(current: &str) -> Option<String> {
    let u = glib::Uri::parse(current, glib::UriFlags::NONE).ok()?;
    let host = untranslate_host(&u.host()?)?;
    Some(rebuild_url(&host, &u, &[]))
}
