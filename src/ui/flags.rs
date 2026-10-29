//! Country flags for language pickers: 48x36 PNG, pre-rendered from flag-icons (MIT, see assets/flags/LICENSE).
//! Language -> flag is approximate (a language has no single country); "auto" and "eo" have no flag.

use crate::*;
use std::collections::HashMap;

static F_AF: &[u8] = include_bytes!("../../assets/flags/af.png");
static F_AL: &[u8] = include_bytes!("../../assets/flags/al.png");
static F_AM: &[u8] = include_bytes!("../../assets/flags/am.png");
static F_AZ: &[u8] = include_bytes!("../../assets/flags/az.png");
static F_BA: &[u8] = include_bytes!("../../assets/flags/ba.png");
static F_BD: &[u8] = include_bytes!("../../assets/flags/bd.png");
static F_BG: &[u8] = include_bytes!("../../assets/flags/bg.png");
static F_BI: &[u8] = include_bytes!("../../assets/flags/bi.png");
static F_BO: &[u8] = include_bytes!("../../assets/flags/bo.png");
static F_BT: &[u8] = include_bytes!("../../assets/flags/bt.png");
static F_BW: &[u8] = include_bytes!("../../assets/flags/bw.png");
static F_BY: &[u8] = include_bytes!("../../assets/flags/by.png");
static F_CD: &[u8] = include_bytes!("../../assets/flags/cd.png");
static F_CF: &[u8] = include_bytes!("../../assets/flags/cf.png");
static F_CN: &[u8] = include_bytes!("../../assets/flags/cn.png");
static F_CZ: &[u8] = include_bytes!("../../assets/flags/cz.png");
static F_DE: &[u8] = include_bytes!("../../assets/flags/de.png");
static F_DK: &[u8] = include_bytes!("../../assets/flags/dk.png");
static F_EE: &[u8] = include_bytes!("../../assets/flags/ee.png");
static F_ER: &[u8] = include_bytes!("../../assets/flags/er.png");
static F_ES: &[u8] = include_bytes!("../../assets/flags/es.png");
static F_ES_CT: &[u8] = include_bytes!("../../assets/flags/es-ct.png");
static F_ES_GA: &[u8] = include_bytes!("../../assets/flags/es-ga.png");
static F_ES_PV: &[u8] = include_bytes!("../../assets/flags/es-pv.png");
static F_ET: &[u8] = include_bytes!("../../assets/flags/et.png");
static F_FI: &[u8] = include_bytes!("../../assets/flags/fi.png");
static F_FJ: &[u8] = include_bytes!("../../assets/flags/fj.png");
static F_FO: &[u8] = include_bytes!("../../assets/flags/fo.png");
static F_FR: &[u8] = include_bytes!("../../assets/flags/fr.png");
static F_GB: &[u8] = include_bytes!("../../assets/flags/gb.png");
static F_GB_WLS: &[u8] = include_bytes!("../../assets/flags/gb-wls.png");
static F_GE: &[u8] = include_bytes!("../../assets/flags/ge.png");
static F_GH: &[u8] = include_bytes!("../../assets/flags/gh.png");
static F_GR: &[u8] = include_bytes!("../../assets/flags/gr.png");
static F_GU: &[u8] = include_bytes!("../../assets/flags/gu.png");
static F_HR: &[u8] = include_bytes!("../../assets/flags/hr.png");
static F_HT: &[u8] = include_bytes!("../../assets/flags/ht.png");
static F_HU: &[u8] = include_bytes!("../../assets/flags/hu.png");
static F_ID: &[u8] = include_bytes!("../../assets/flags/id.png");
static F_IE: &[u8] = include_bytes!("../../assets/flags/ie.png");
static F_IL: &[u8] = include_bytes!("../../assets/flags/il.png");
static F_IM: &[u8] = include_bytes!("../../assets/flags/im.png");
static F_IN: &[u8] = include_bytes!("../../assets/flags/in.png");
static F_IQ: &[u8] = include_bytes!("../../assets/flags/iq.png");
static F_IR: &[u8] = include_bytes!("../../assets/flags/ir.png");
static F_IS: &[u8] = include_bytes!("../../assets/flags/is.png");
static F_IT: &[u8] = include_bytes!("../../assets/flags/it.png");
static F_JP: &[u8] = include_bytes!("../../assets/flags/jp.png");
static F_KG: &[u8] = include_bytes!("../../assets/flags/kg.png");
static F_KH: &[u8] = include_bytes!("../../assets/flags/kh.png");
static F_KR: &[u8] = include_bytes!("../../assets/flags/kr.png");
static F_KZ: &[u8] = include_bytes!("../../assets/flags/kz.png");
static F_LA: &[u8] = include_bytes!("../../assets/flags/la.png");
static F_LK: &[u8] = include_bytes!("../../assets/flags/lk.png");
static F_LS: &[u8] = include_bytes!("../../assets/flags/ls.png");
static F_LT: &[u8] = include_bytes!("../../assets/flags/lt.png");
static F_LU: &[u8] = include_bytes!("../../assets/flags/lu.png");
static F_LV: &[u8] = include_bytes!("../../assets/flags/lv.png");
static F_MG: &[u8] = include_bytes!("../../assets/flags/mg.png");
static F_MK: &[u8] = include_bytes!("../../assets/flags/mk.png");
static F_ML: &[u8] = include_bytes!("../../assets/flags/ml.png");
static F_MM: &[u8] = include_bytes!("../../assets/flags/mm.png");
static F_MN: &[u8] = include_bytes!("../../assets/flags/mn.png");
static F_MT: &[u8] = include_bytes!("../../assets/flags/mt.png");
static F_MV: &[u8] = include_bytes!("../../assets/flags/mv.png");
static F_MW: &[u8] = include_bytes!("../../assets/flags/mw.png");
static F_MY: &[u8] = include_bytes!("../../assets/flags/my.png");
static F_NG: &[u8] = include_bytes!("../../assets/flags/ng.png");
static F_NL: &[u8] = include_bytes!("../../assets/flags/nl.png");
static F_NO: &[u8] = include_bytes!("../../assets/flags/no.png");
static F_NP: &[u8] = include_bytes!("../../assets/flags/np.png");
static F_NZ: &[u8] = include_bytes!("../../assets/flags/nz.png");
static F_PE: &[u8] = include_bytes!("../../assets/flags/pe.png");
static F_PH: &[u8] = include_bytes!("../../assets/flags/ph.png");
static F_PK: &[u8] = include_bytes!("../../assets/flags/pk.png");
static F_PL: &[u8] = include_bytes!("../../assets/flags/pl.png");
static F_PT: &[u8] = include_bytes!("../../assets/flags/pt.png");
static F_PY: &[u8] = include_bytes!("../../assets/flags/py.png");
static F_RO: &[u8] = include_bytes!("../../assets/flags/ro.png");
static F_RS: &[u8] = include_bytes!("../../assets/flags/rs.png");
static F_RU: &[u8] = include_bytes!("../../assets/flags/ru.png");
static F_RW: &[u8] = include_bytes!("../../assets/flags/rw.png");
static F_SA: &[u8] = include_bytes!("../../assets/flags/sa.png");
static F_SE: &[u8] = include_bytes!("../../assets/flags/se.png");
static F_SI: &[u8] = include_bytes!("../../assets/flags/si.png");
static F_SK: &[u8] = include_bytes!("../../assets/flags/sk.png");
static F_SN: &[u8] = include_bytes!("../../assets/flags/sn.png");
static F_SO: &[u8] = include_bytes!("../../assets/flags/so.png");
static F_SZ: &[u8] = include_bytes!("../../assets/flags/sz.png");
static F_TH: &[u8] = include_bytes!("../../assets/flags/th.png");
static F_TJ: &[u8] = include_bytes!("../../assets/flags/tj.png");
static F_TM: &[u8] = include_bytes!("../../assets/flags/tm.png");
static F_TO: &[u8] = include_bytes!("../../assets/flags/to.png");
static F_TR: &[u8] = include_bytes!("../../assets/flags/tr.png");
static F_TZ: &[u8] = include_bytes!("../../assets/flags/tz.png");
static F_UA: &[u8] = include_bytes!("../../assets/flags/ua.png");
static F_UZ: &[u8] = include_bytes!("../../assets/flags/uz.png");
static F_VA: &[u8] = include_bytes!("../../assets/flags/va.png");
static F_VN: &[u8] = include_bytes!("../../assets/flags/vn.png");
static F_WS: &[u8] = include_bytes!("../../assets/flags/ws.png");
static F_ZA: &[u8] = include_bytes!("../../assets/flags/za.png");
static F_ZW: &[u8] = include_bytes!("../../assets/flags/zw.png");

/// PNG bytes of the flag used for a language code (None for "auto", "eo" and unknown codes).
pub(crate) fn lang_flag_bytes(lang: &str) -> Option<&'static [u8]> {
    Some(match lang {
        "uz" => F_UZ,
        "en" => F_GB,
        "ru" => F_RU,
        "ab" => F_GE,
        "aa" => F_ET,
        "af" => F_ZA,
        "sq" => F_AL,
        "am" => F_ET,
        "ar" => F_SA,
        "hy" => F_AM,
        "as" => F_IN,
        "av" => F_RU,
        "ay" => F_BO,
        "az" => F_AZ,
        "bm" => F_ML,
        "ba" => F_RU,
        "eu" => F_ES_PV,
        "be" => F_BY,
        "bn" => F_BD,
        "bs" => F_BA,
        "br" => F_FR,
        "bg" => F_BG,
        "my" => F_MM,
        "ca" => F_ES_CT,
        "ch" => F_GU,
        "ce" => F_RU,
        "ny" => F_MW,
        "zh" => F_CN,
        "cv" => F_RU,
        "co" => F_FR,
        "hr" => F_HR,
        "cs" => F_CZ,
        "da" => F_DK,
        "dv" => F_MV,
        "nl" => F_NL,
        "dz" => F_BT,
        "et" => F_EE,
        "ee" => F_GH,
        "fo" => F_FO,
        "fj" => F_FJ,
        "fi" => F_FI,
        "fr" => F_FR,
        "fy" => F_NL,
        "ff" => F_SN,
        "gl" => F_ES_GA,
        "ka" => F_GE,
        "de" => F_DE,
        "el" => F_GR,
        "gn" => F_PY,
        "gu" => F_IN,
        "ht" => F_HT,
        "ha" => F_NG,
        "he" => F_IL,
        "hi" => F_IN,
        "hu" => F_HU,
        "is" => F_IS,
        "ig" => F_NG,
        "id" => F_ID,
        "ga" => F_IE,
        "it" => F_IT,
        "ja" => F_JP,
        "jv" => F_ID,
        "kn" => F_IN,
        "kr" => F_NG,
        "ks" => F_IN,
        "kk" => F_KZ,
        "km" => F_KH,
        "rw" => F_RW,
        "ky" => F_KG,
        "kv" => F_RU,
        "kg" => F_CD,
        "ko" => F_KR,
        "ku" => F_IQ,
        "lo" => F_LA,
        "la" => F_VA,
        "lv" => F_LV,
        "li" => F_NL,
        "ln" => F_CD,
        "lt" => F_LT,
        "lb" => F_LU,
        "mk" => F_MK,
        "mg" => F_MG,
        "ms" => F_MY,
        "ml" => F_IN,
        "mt" => F_MT,
        "gv" => F_IM,
        "mi" => F_NZ,
        "mr" => F_IN,
        "mn" => F_MN,
        "ne" => F_NP,
        "no" => F_NO,
        "or" => F_IN,
        "om" => F_ET,
        "os" => F_RU,
        "ps" => F_AF,
        "fa" => F_IR,
        "pl" => F_PL,
        "pt" => F_PT,
        "pa" => F_IN,
        "qu" => F_PE,
        "ro" => F_RO,
        "rn" => F_BI,
        "sm" => F_WS,
        "sg" => F_CF,
        "sa" => F_IN,
        "sr" => F_RS,
        "sn" => F_ZW,
        "sd" => F_PK,
        "si" => F_LK,
        "sk" => F_SK,
        "sl" => F_SI,
        "so" => F_SO,
        "st" => F_LS,
        "es" => F_ES,
        "su" => F_ID,
        "sw" => F_TZ,
        "ss" => F_SZ,
        "sv" => F_SE,
        "tl" => F_PH,
        "tg" => F_TJ,
        "ta" => F_IN,
        "tt" => F_RU,
        "te" => F_IN,
        "th" => F_TH,
        "bo" => F_CN,
        "ti" => F_ER,
        "to" => F_TO,
        "ts" => F_ZA,
        "tn" => F_BW,
        "tr" => F_TR,
        "tk" => F_TM,
        "tw" => F_GH,
        "ug" => F_CN,
        "uk" => F_UA,
        "ur" => F_PK,
        "ve" => F_ZA,
        "vi" => F_VN,
        "cy" => F_GB_WLS,
        "wo" => F_SN,
        "xh" => F_ZA,
        "yi" => F_IL,
        "yo" => F_NG,
        "zu" => F_ZA,
        _ => return None,
    })
}

// Auto-detect language icon: icons/svg/auto-lang.svg

thread_local! {
    static FLAG_CACHE: RefCell<HashMap<&'static str, Option<gdk::Texture>>> = RefCell::new(HashMap::new());
}

/// Decoded flag texture for a language code, cached (decoded once, only when a row is shown).
pub(crate) fn lang_flag_texture(lang: &'static str) -> Option<gdk::Texture> {
    FLAG_CACHE.with(|c| {
        c.borrow_mut()
            .entry(lang)
            .or_insert_with(|| {
                let bytes = if lang == "auto" {
                    svg_bytes("auto-lang")?
                } else {
                    lang_flag_bytes(lang)?
                };
                gdk::Texture::from_bytes(&glib::Bytes::from_static(bytes)).ok()
            })
            .clone()
    })
}
