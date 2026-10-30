//! Rail i18n: O'zbekcha (default), English, Русский.
//! Matnlar inglizcha kalit bo'yicha tarjima qilinadi; topilmasa inglizcha qoladi.
//! Til o'zgarishi Rail qayta ochilganda kuchga kiradi (til data papkadagi language.txt da).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Uz,
    En,
    Ru,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::Uz => "uz",
            Lang::En => "en",
            Lang::Ru => "ru",
        }
    }
    pub fn from_code(s: &str) -> Option<Lang> {
        match s.trim().to_lowercase().as_str() {
            "uz" => Some(Lang::Uz),
            "en" => Some(Lang::En),
            "ru" => Some(Lang::Ru),
            _ => None,
        }
    }
}

fn lang_file() -> PathBuf {
    crate::rail_data_dir().join("language.txt")
}

/// Joriy til: language.txt, bo'lmasa o'zbekcha.
pub fn current() -> Lang {
    static L: OnceLock<Lang> = OnceLock::new();
    *L.get_or_init(|| {
        std::fs::read_to_string(lang_file())
            .ok()
            .and_then(|s| Lang::from_code(&s))
            .unwrap_or(Lang::Uz)
    })
}

/// Tilni saqlaydi (qayta ochilganda kuchga kiradi).
pub fn set_language(l: Lang) -> bool {
    let p = lang_file();
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(p, l.code()).is_ok()
}

fn table(l: Lang) -> &'static HashMap<&'static str, &'static str> {
    static UZ: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    static RU: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    match l {
        Lang::Ru => RU.get_or_init(|| {
            crate::i18n_tables::TABLE
                .iter()
                .map(|(en, _, ru)| (*en, *ru))
                .collect()
        }),
        _ => UZ.get_or_init(|| {
            crate::i18n_tables::TABLE
                .iter()
                .map(|(en, uz, _)| (*en, *uz))
                .collect()
        }),
    }
}

/// Tarjima: inglizcha kalit -> joriy til. Topilmasa kalitning o'zi.
pub fn tr(s: &'static str) -> &'static str {
    match current() {
        Lang::En => s,
        l => table(l).get(s).copied().unwrap_or(s),
    }
}

/// Tarjima + "{}" joylarini ketma-ket args bilan to'ldirish.
pub fn trf(tpl: &'static str, args: &[&str]) -> String {
    let t = tr(tpl);
    let mut out = String::with_capacity(t.len() + 16);
    let mut it = args.iter();
    let mut rest = t;
    while let Some(pos) = rest.find("{}") {
        out.push_str(&rest[..pos]);
        if let Some(a) = it.next() {
            out.push_str(a);
        }
        rest = &rest[pos + 2..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_complete_and_unique() {
        let mut seen = std::collections::HashSet::new();
        for (en, uz, ru) in crate::i18n_tables::TABLE {
            assert!(
                !en.is_empty() && !uz.is_empty() && !ru.is_empty(),
                "bo'sh: {en}"
            );
            assert!(seen.insert(*en), "takror kalit: {en}");
            assert_eq!(
                en.matches("{}").count(),
                uz.matches("{}").count(),
                "uz {{}} soni: {en}"
            );
            assert_eq!(
                en.matches("{}").count(),
                ru.matches("{}").count(),
                "ru {{}} soni: {en}"
            );
        }
    }

    #[test]
    fn trf_fills_placeholders() {
        assert_eq!(trf("a {} b {}", &["1", "2"]), "a 1 b 2");
    }

    /// Manba kodidagi har bir tr("...") / trf("...") kaliti jadvalda bo'lishi shart.
    #[test]
    fn every_source_key_is_in_table() {
        let keys: std::collections::HashSet<&str> = crate::i18n_tables::TABLE
            .iter()
            .map(|(en, _, _)| *en)
            .collect();
        let mut missing = Vec::new();
        for (file, src) in [
            ("main.rs", include_str!("main.rs")),
            ("update.rs", include_str!("update.rs")),
            ("welcome.rs", include_str!("app/welcome.rs")),
            ("start.rs", include_str!("pages/start.rs")),
        ] {
            let b = src.as_bytes();
            let mut i = 0;
            while i < b.len() {
                let (skip, ok) = if b[i..].starts_with(b"trf(\"") {
                    (
                        5,
                        i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_'),
                    )
                } else if b[i..].starts_with(b"tr(\"") {
                    (
                        4,
                        i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_'),
                    )
                } else {
                    (0, false)
                };
                if !ok {
                    i += 1;
                    continue;
                }
                let mut j = i + skip;
                let mut lit = String::new();
                while j < b.len() && b[j] != b'"' {
                    if b[j] == b'\\' {
                        j += 1;
                        match b[j] {
                            b'n' => lit.push('\n'),
                            b't' => lit.push('\t'),
                            c => lit.push(c as char),
                        }
                        j += 1;
                    } else {
                        let ch = src[j..].chars().next().unwrap();
                        lit.push(ch);
                        j += ch.len_utf8();
                    }
                }
                if !keys.contains(lit.as_str()) {
                    missing.push(format!("{file}: {lit}"));
                }
                i = j + 1;
            }
        }
        assert!(
            missing.is_empty(),
            "jadvalda yo'q kalitlar:\n{}",
            missing.join("\n")
        );
    }
}
