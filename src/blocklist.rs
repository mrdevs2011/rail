//! Ad and tracker blocking through WebKit's content filters.
//!
//! Third-party requests to known ad/tracker hosts below are blocked. Only
//! high-confidence ad markers are hidden; generic class names are deliberately
//! avoided because sites often reuse names such as `.advertisement` for real UI.
//! Put extra hosts (one per line) in `blocklist.txt`.

const HOSTS: &[&str] = &[
    // ad networks & exchanges
    "doubleclick.net",
    "googlesyndication.com",
    "googleadservices.com",
    "adservice.google.com",
    "adnxs.com",
    "adsrvr.org",
    "advertising.com",
    "adform.net",
    "criteo.com",
    "criteo.net",
    "taboola.com",
    "outbrain.com",
    "revcontent.com",
    "mgid.com",
    "pubmatic.com",
    "rubiconproject.com",
    "openx.net",
    "casalemedia.com",
    "indexww.com",
    "sharethrough.com",
    "smartadserver.com",
    "yieldmo.com",
    "media.net",
    "amazon-adsystem.com",
    "adsafeprotected.com",
    "moatads.com",
    "2mdn.net",
    "serving-sys.com",
    "bidswitch.net",
    "contextweb.com",
    "lijit.com",
    "sovrn.com",
    "teads.tv",
    "33across.com",
    "gumgum.com",
    "triplelift.com",
    "tapad.com",
    "zemanta.com",
    "adcolony.com",
    "applovin.com",
    "inmobi.com",
    "mopub.com",
    "popads.net",
    "propellerads.com",
    "exoclick.com",
    "juicyads.com",
    "trafficjunky.net",
    "adskeeper.com",
    "ad.gt",
    "adition.com",
    // analytics & tracking
    "google-analytics.com",
    "googletagmanager.com",
    "googletagservices.com",
    "scorecardresearch.com",
    "quantserve.com",
    "quantcount.com",
    "chartbeat.com",
    "chartbeat.net",
    "hotjar.com",
    "hotjar.io",
    "mouseflow.com",
    "crazyegg.com",
    "fullstory.com",
    "luckyorange.com",
    "inspectlet.com",
    "mixpanel.com",
    "segment.io",
    "segment.com",
    "amplitude.com",
    "heap.io",
    "heapanalytics.com",
    "newrelic.com",
    "nr-data.net",
    "bugsnag.com",
    "branch.io",
    "appsflyer.com",
    "adjust.com",
    "kochava.com",
    "bluekai.com",
    "demdex.net",
    "everesttech.net",
    "omtrdc.net",
    "2o7.net",
    "krxd.net",
    "agkn.com",
    "rlcdn.com",
    "liveramp.com",
    "id5-sync.com",
    "crwdcntrl.net",
    "exelator.com",
    "eyeota.net",
    "mathtag.com",
    "turn.com",
    "mookie1.com",
    "rfihub.com",
    "pippio.com",
    "bidr.io",
    "dotomi.com",
    "yimg.com/aaq",
    "ads-twitter.com",
    "analytics.twitter.com",
    "px.ads.linkedin.com",
    "snap.licdn.com",
    "analytics.tiktok.com",
    "ads.tiktok.com",
    "pixel.facebook.com",
    "an.facebook.com",
    "analytics.yahoo.com",
    "ads.yahoo.com",
    "log.pinterest.com",
    "ct.pinterest.com",
    "tr.snapchat.com",
    "sc-static.net",
    "clarity.ms",
    "bat.bing.com",
    "optimizely.com",
    "vwo.com",
    "abtasty.com",
    "tiqcdn.com",
    "browser-update.org",
    "outbrainimg.com",
    "disqusads.com",
    "ml314.com",
    "tags.tiqcdn.com",
    "cdn.taboola.com",
    "trc.taboola.com",
    "sb.scorecardresearch.com",
    "pagead2.googlesyndication.com",
];

/// Obvious ad containers; kept short on purpose so pages are not broken.
const HIDE: &str = "ins.adsbygoogle, .adsbygoogle, [id^='google_ads_iframe'], [id^='div-gpt-ad'], \
[id^='google_ads_'], iframe[src*='doubleclick.net'], iframe[src*='googlesyndication.com'], \
[data-ad-slot], [data-google-query-id]";

fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push(' '),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn host_rule(host: &str) -> String {
    let escaped = host.replace('.', "\\.");
    let filter = format!("^[^:]+://+([^:/]+\\.)?{escaped}[:/]");
    format!(
        "{{\"trigger\":{{\"url-filter\":{},\"load-type\":[\"third-party\"]}},\"action\":{{\"type\":\"block\"}}}}",
        json_str(&filter)
    )
}

/// Identity and core site hosts must never be added to the block list. Several of these
/// domains also serve OAuth, profile photos, site assets and federated sign-in scripts.
const PROTECTED_HOSTS: &[&str] = &[
    "github.com",
    "githubassets.com",
    "githubusercontent.com",
    "google.com",
    "googleusercontent.com",
    "gstatic.com",
    "apple.com",
    "icloud.com",
    "microsoft.com",
    "microsoftonline.com",
    "live.com",
    "outlook.com",
    "office.com",
    "facebook.com",
    "connect.facebook.net",
];

fn is_protected_host(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    PROTECTED_HOSTS
        .iter()
        .any(|protected| host == *protected || host.ends_with(&format!(".{protected}")))
}

/// The rule list in WebKit's content-blocker JSON format.
pub fn rules_json(extra: &str) -> String {
    let mut rules: Vec<String> = HOSTS
        .iter()
        .filter(|h| !h.contains('/') && !is_protected_host(h))
        .map(|h| host_rule(h))
        .collect();
    for line in extra.lines() {
        let h = line
            .split('#')
            .next()
            .unwrap_or("")
            .trim()
            .trim_start_matches("*.");
        if !h.is_empty()
            && h.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
            && !is_protected_host(h)
        {
            rules.push(host_rule(h));
        }
    }
    rules.push(format!(
        "{{\"trigger\":{{\"url-filter\":\".*\"}},\"action\":{{\"type\":\"css-display-none\",\"selector\":{}}}}}",
        json_str(HIDE)
    ));
    format!("[{}]", rules.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_only_high_confidence_ad_markers() {
        let rules = rules_json("");
        assert!(rules.contains("ins.adsbygoogle"));
        assert!(rules.contains("[data-ad-slot]"));
        assert!(!rules.contains(".ad-container"));
        assert!(!rules.contains(".advertisement"));
        assert!(!rules.contains(".advert,"));
        assert!(!rules.contains("\"url-filter\":\"/pagead/ads\""));
        assert!(!rules.contains("\"url-filter\":\"/pagead/js\""));
        for protected_host in [
            "github\\.com",
            "githubassets\\.com",
            "githubusercontent\\.com",
            "accounts\\.google\\.com",
            "gstatic\\.com",
            "appleid\\.apple\\.com",
            "apple\\.com",
            "icloud\\.com",
        ] {
            assert!(
                !rules.contains(protected_host),
                "identity host was blocked: {protected_host}"
            );
        }
        let protected_custom = rules_json(
            "github.com\naccounts.google.com\nappleid.apple.com\nconnect.facebook.net\ntracker.example",
        );
        assert!(!protected_custom.contains("github\\.com"));
        assert!(!protected_custom.contains("accounts\\.google"));
        assert!(!protected_custom.contains("appleid\\.apple"));
        assert!(!protected_custom.contains("connect\\.facebook"));
        assert!(protected_custom.contains("tracker"));
        assert!(
            crate::minijson::parse(&rules).is_some(),
            "content-blocker JSON must remain valid"
        );
    }
}
