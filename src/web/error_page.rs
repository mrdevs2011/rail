//! Developer-grade, dark (#000000) trilingual error page with full technical categorization.
//! Covers Chromium net::ERR_*, W3C DOMExceptions, Web APIs, and JS Runtime errors.

pub(crate) struct ErrorDetail {
    pub code: &'static str,
    pub category: &'static str,
    pub badge_color: &'static str,
    pub title_uz: &'static str,
    pub title_ru: &'static str,
    pub title_en: &'static str,
    pub desc_uz: &'static str,
    pub desc_ru: &'static str,
    pub desc_en: &'static str,
    pub fixes_uz: &'static [&'static str],
    pub fixes_ru: &'static [&'static str],
    pub fixes_en: &'static [&'static str],
}

pub(crate) fn match_error_info(msg: &str) -> ErrorDetail {
    let lower = msg.to_lowercase();

    // 1. Connection Refused (port down, localhost stopped)
    if lower.contains("connection refused") || lower.contains("refused") {
        return ErrorDetail {
            code: "net::ERR_CONNECTION_REFUSED",
            category: "Network / Local Server",
            badge_color: "#f59e0b",
            title_uz: "Ulanish rad etildi",
            title_ru: "В соединении отказано",
            title_en: "Connection Refused",
            desc_uz: "Ko'rsatilgan portda hech qanday servis yoki server tinglamayapti. Mahalliy dasturlash serveri (localhost) o'chirilgan yoki port noto'g'ri ko'rsatilgan bo'lishi mumkin.",
            desc_ru: "По указанному порту или адресу не запущена служба. Локальный сервер разработки (localhost) может быть остановлен или указан неверный порт.",
            desc_en: "No active service or server is listening on the specified port. Your local development server may not be running or the port is incorrect.",
            fixes_uz: &[
                "Terminalda loyiha serveri ishlayotganini tekshiring (masalan: `npm run dev`, `yarn dev`, `python manage.py runserver`, `cargo run`).",
                "Manzildagi port raqamini tekshiring (odatda `:3000`, `:5173`, `:8000`, `:8080`).",
                "Firewall yoki lokal proksi ulanishni to'smayotganligiga ishonch hosil qiling."
            ],
            fixes_ru: &[
                "Убедитесь, что сервер проекта запущен в терминале (напр.: `npm run dev`, `python manage.py runserver`, `cargo run`).",
                "Проверьте правильность порта в адресе (обычно `:3000`, `:5173`, `:8000`, `:8080`).",
                "Проверьте, не блокирует ли соединение брандмауэр или локальный прокси."
            ],
            fixes_en: &[
                "Verify your project dev server is running in terminal (e.g. `npm run dev`, `python manage.py runserver`, `cargo run`).",
                "Double check the port number in the address bar (typically `:3000`, `:5173`, `:8000`, `:8080`).",
                "Ensure local firewall or proxy settings are not blocking the loopback address."
            ],
        };
    }

    // 2. DNS / Name Not Resolved
    if lower.contains("resolve") || lower.contains("not found") || lower.contains("name_not_resolved") || lower.contains("cannot find") {
        return ErrorDetail {
            code: "net::ERR_NAME_NOT_RESOLVED",
            category: "DNS / Name Resolution",
            badge_color: "#38bdf8",
            title_uz: "Domen nomi topilmadi",
            title_ru: "Имя домена не найдено",
            title_en: "Server IP Address Could Not Be Found",
            desc_uz: "DNS server ushbu domen nomi bo'yicha IP manzilni aniqlay olmadi. Domen nomi noto'g'ri yozilgan yoki internet aloqasida uzilish bor.",
            desc_ru: "DNS-сервер не смог разрешить IP-адрес для этого доменного имени. Имя домена введено с ошибкой или отсутствует интернет-соединение.",
            desc_en: "The DNS server could not resolve an IP address for this host. The domain may be misspelled or your internet connection is down.",
            fixes_uz: &[
                "Domen nomining yozilishida harfiy xatolar yo'qligini tekshiring.",
                "Internet ulanishingiz va Wi-Fi holatini tekshiring.",
                "DNS server sozlamalarini (masalan: 1.1.1.1 yoki 8.8.8.8) tekshiring yoki DNS keshini tozalang."
            ],
            fixes_ru: &[
                "Проверьте правильность написания доменного имени в строке адреса.",
                "Проверьте подключение к интернету и статус сети.",
                "Попробуйте сменить DNS-сервер (напр.: 1.1.1.1 или 8.8.8.8) или очистить DNS-кэш."
            ],
            fixes_en: &[
                "Check the URL for spelling mistakes or typos.",
                "Check your network and Wi-Fi connection.",
                "Verify your DNS resolver settings (e.g. 1.1.1.1 or 8.8.8.8) or flush DNS cache."
            ],
        };
    }

    // 3. Connection Timed Out
    if lower.contains("timed out") || lower.contains("timeout") {
        return ErrorDetail {
            code: "net::ERR_CONNECTION_TIMED_OUT",
            category: "Network / Timeout",
            badge_color: "#f59e0b",
            title_uz: "Ulanish vaqti tugadi",
            title_ru: "Время ожидания ответа истекло",
            title_en: "Connection Timed Out",
            desc_uz: "Server belgilangan vaqt ichida javob bermadi. Server ortiqcha yuklama ostida bo'lishi yoki tarmoq yo'nalishida uzilish yuz bergan.",
            desc_ru: "Сервер не ответил за отведенное время. Возможно, сервер перегружен или возникли сетевые задержки.",
            desc_en: "The server took too long to respond. It may be overloaded, offline, or network packets are being dropped.",
            fixes_uz: &[
                "Sahifani bir necha soniyadan so'ng qayta yuklab ko'ring.",
                "Internet tezligi va proksi ulanishini tekshiring.",
                "Agar bu tashqi API bo'lsa, xizmat holatini (status page) tekshiring."
            ],
            fixes_ru: &[
                "Попробуйте перезагрузить страницу через несколько секунд.",
                "Проверьте стабильность интернет-соединения и настройки прокси.",
                "Если это внешний сервис, проверьте его статус доступности."
            ],
            fixes_en: &[
                "Try reloading the page in a few moments.",
                "Check your internet latency and VPN/proxy connection.",
                "If using a third-party API, check their status page for outages."
            ],
        };
    }

    // 4. Connection Reset / Closed
    if lower.contains("reset") || lower.contains("closed") {
        return ErrorDetail {
            code: "net::ERR_CONNECTION_RESET",
            category: "Network / TCP Reset",
            badge_color: "#f59e0b",
            title_uz: "Ulanish uzildi",
            title_ru: "Соединение сброшено",
            title_en: "Connection Was Reset",
            desc_uz: "Server yoki oraliq tarmoq uzeli bilan TCP aloqasi kutilmaganda uzildi (RST paketi qabul qilindi).",
            desc_ru: "Сетевое соединение с сервером было внезапно прервано (получен пакет RST).",
            desc_en: "The connection to the server was unexpectedly closed or reset during communication.",
            fixes_uz: &[
                "Server jarayoni qulab tushmaganligini (crash) tekshiring.",
                "VPN yoki xavfsizlik devori (firewall) ulanishni to'xtatmaganligiga ishonch hosil qiling."
            ],
            fixes_ru: &[
                "Убедитесь, что серверный процесс не упал с ошибкой.",
                "Проверьте настройки VPN или корпоративного файрвола."
            ],
            fixes_en: &[
                "Check your backend logs to verify the server didn't crash.",
                "Verify that a VPN or corporate firewall is not terminating the socket."
            ],
        };
    }

    // 5. SSL / TLS Certificate Errors
    if lower.contains("certificate") || lower.contains("tls") || lower.contains("ssl") {
        return ErrorDetail {
            code: "net::ERR_CERT_AUTHORITY_INVALID",
            category: "Security / SSL & TLS",
            badge_color: "#ef4444",
            title_uz: "Xavfsizlik sertifikati xatosi",
            title_ru: "Ошибка сертификата безопасности",
            title_en: "Security Certificate Error",
            desc_uz: "Ushbu saytning HTTPS sertifikati ishonchsiz organ tomonidan berilgan, muddati o'tgan yoki shifrlash kalitlari mos kelmadi.",
            desc_ru: "HTTPS-сертификат сайта не является доверенным, просрочен или не соответствует домену.",
            desc_en: "The website's HTTPS certificate is invalid, expired, self-signed, or issued by an untrusted authority.",
            fixes_uz: &[
                "Kompyuteringiz sana va vaqti to'g'ri sozlanganligini tekshiring.",
                "Lokal HTTPS sinovlari uchun Sozlamalar > Maxfiylik bo'limida 'Developer Security' opsiyasidan foydalaning.",
                "Ishlab chiqaruvchi sertifikatini (CA) yangilang."
            ],
            fixes_ru: &[
                "Проверьте правильность системного времени и даты на компьютере.",
                "Для локальных тестов используйте 'Developer Security' в Настройках > Конфиденциальность.",
                "Обновите SSL-сертификат на веб-сервере."
            ],
            fixes_en: &[
                "Verify your computer's date and time are accurate.",
                "For local testing, use 'Developer Security' overrides in Settings > Privacy.",
                "Ensure valid certificate chains are installed on the web server."
            ],
        };
    }

    // 6. Blocked by Policy / CSP / Shield
    if lower.contains("blocked") || lower.contains("permission") || lower.contains("denied") {
        return ErrorDetail {
            code: "net::ERR_BLOCKED_BY_CLIENT",
            category: "Security / Policy Shield",
            badge_color: "#a855f7",
            title_uz: "So'rov bloklandi",
            title_ru: "Запрос заблокирован",
            title_en: "Request Blocked by Client Policy",
            desc_uz: "Ushbu so'rov brauzerning himoya qalqoni (AdBlock, CSP yoki X-Frame-Options) tomonidan to'xtatildi.",
            desc_ru: "Запрос заблокирован политикой безопасности (AdBlock, CSP или X-Frame-Options).",
            desc_en: "The request was blocked by security policies, AdBlock shield, or Content Security Policy headers.",
            fixes_uz: &[
                "Sozlamalar > Maxfiylik bo'limida kerakli cheklovlarni (AdBlock / CORS) moslang.",
                "Veb-ilovaning CSP sarlavhalarini (Content-Security-Policy) tekshiring."
            ],
            fixes_ru: &[
                "Настройте параметры в Настройках > Конфиденциальность (AdBlock / CORS).",
                "Проверьте заголовки CSP (Content-Security-Policy) вашего веб-приложения."
            ],
            fixes_en: &[
                "Adjust AdBlock or CORS settings in Settings > Privacy & Shield.",
                "Inspect response Content-Security-Policy headers in DevTools (F12)."
            ],
        };
    }

    // Default Fallback: General Network Error
    ErrorDetail {
        code: "net::ERR_NETWORK_ERROR",
        category: "Web & Network",
        badge_color: "#ef4444",
        title_uz: "Sahifani yuklab bo'lmadi",
        title_ru: "Не удалось загрузить страницу",
        title_en: "Page Failed to Load",
        desc_uz: "Brauzer ushbu veb-manzilni yuklashda texnik xatolikka duch keldi.",
        desc_ru: "Браузер столкнулся с технической ошибкой при загрузке адреса.",
        desc_en: "The browser encountered an error while attempting to load this address.",
        fixes_uz: &[
            "Manzil to'g'ri kiritilganligiga ishonch hosil qiling.",
            "Qurilmangiz internet tarmog'iga ulanganligini tekshiring.",
            "F12 (DevTools) orqali Network va Console oynasidagi batafsil loglarni ko'ring."
        ],
        fixes_ru: &[
            "Убедитесь в правильности введенного адреса.",
            "Проверьте сетевое подключение вашего устройства.",
            "Откройте DevTools (F12) для просмотра подробных логов в Network и Console."
        ],
        fixes_en: &[
            "Verify that the web address is entered correctly.",
            "Check that your device has an active internet connection.",
            "Press F12 to inspect detailed logs in Network and Console tabs."
        ],
    }
}

pub(crate) fn format_error_html(failing_uri: &str, raw_error: &str) -> String {
    let detail = match_error_info(raw_error);
    let lang = std::fs::read_to_string(crate::rail_data_dir().join("language.txt"))
        .ok()
        .and_then(|s| crate::i18n::Lang::from_code(&s))
        .unwrap_or_else(crate::i18n::current);

    let safe_uri = failing_uri.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");
    let safe_raw = raw_error.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let timestamp = gtk4::glib::DateTime::now_local()
        .ok()
        .and_then(|d| d.format("%Y-%m-%d %H:%M:%S UTC%z").ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Just now".to_string());

    let (lang_code, title, desc, fixes, fixtitle, reload_lbl, home_lbl, copy_lbl, copied_lbl, diag_lbl) = match lang {
        crate::i18n::Lang::Ru => (
            "ru",
            detail.title_ru,
            detail.desc_ru,
            detail.fixes_ru,
            "Рекомендуемые действия:",
            "Перезагрузить",
            "На главную",
            "Копировать",
            "✓ Скопировано!",
            "Технические сведения об ошибке (DevTools)",
        ),
        crate::i18n::Lang::En => (
            "en",
            detail.title_en,
            detail.desc_en,
            detail.fixes_en,
            "Suggested Troubleshooting:",
            "Try Again",
            "Home Page",
            "Copy Report",
            "✓ Copied!",
            "Technical Diagnostics (DevTools)",
        ),
        crate::i18n::Lang::Uz => (
            "uz",
            detail.title_uz,
            detail.desc_uz,
            detail.fixes_uz,
            "Tavsiya etilgan yechimlar:",
            "Qayta yuklash",
            "Bosh sahifa",
            "Nusxa olish",
            "✓ Nusxalandi!",
            "Texnik diagnostika ma'lumotlari (DevTools)",
        ),
    };

    let mut fixes_html = String::new();
    for f in fixes {
        fixes_html.push_str(&format!("<li>{}</li>", f));
    }

    format!(r##"<!doctype html>
<html lang="{lang_code}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{code} - Rail Browser</title>
<style>
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  html, body {{
    background-color: #000000;
    color: #e4e4e7;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 24px 16px;
    user-select: text;
  }}
  .error-card {{
    background: #0d0e12;
    border: 1px solid #1f232b;
    border-radius: 14px;
    max-width: 680px;
    width: 100%;
    padding: 36px 32px;
    box-shadow: 0 20px 45px rgba(0,0,0,0.85);
  }}
  .top-row {{
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 20px;
    flex-wrap: wrap;
    gap: 12px;
  }}
  .badge-category {{
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    padding: 4px 10px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
  }}
  .badge-dot {{
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: {badge_color};
    box-shadow: 0 0 8px {badge_color};
  }}
  .error-code {{
    font-family: ui-monospace, "JetBrains Mono", Consolas, monospace;
    font-size: 13px;
    font-weight: 600;
    color: {badge_color};
    margin-bottom: 8px;
    letter-spacing: 0.2px;
  }}
  .error-title {{
    font-size: 24px;
    font-weight: 800;
    color: #ffffff;
    line-height: 1.3;
    margin-bottom: 12px;
    letter-spacing: -0.3px;
  }}
  .error-uri {{
    display: inline-block;
    font-family: ui-monospace, "JetBrains Mono", Consolas, monospace;
    font-size: 12px;
    color: #94a3b8;
    background: #141720;
    border: 1px solid #232733;
    padding: 5px 10px;
    border-radius: 6px;
    margin-bottom: 20px;
    word-break: break-all;
    max-width: 100%;
  }}
  .error-desc {{
    font-size: 14px;
    line-height: 1.6;
    color: #cbd5e1;
    margin-bottom: 24px;
  }}
  .fixes-card {{
    background: #12141a;
    border-left: 3px solid {badge_color};
    border-radius: 0 8px 8px 0;
    padding: 16px 18px;
    margin-bottom: 28px;
  }}
  .fixes-title {{
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #94a3b8;
    margin-bottom: 10px;
  }}
  .fixes-list {{
    font-size: 13px;
    line-height: 1.6;
    color: #e2e8f0;
    padding-left: 18px;
  }}
  .fixes-list li {{
    margin-bottom: 6px;
  }}
  .fixes-list code {{
    font-family: ui-monospace, "JetBrains Mono", Consolas, monospace;
    background: #1e222d;
    padding: 2px 6px;
    border-radius: 4px;
    color: #38bdf8;
    font-size: 12px;
  }}
  .actions-row {{
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    margin-bottom: 24px;
  }}
  .btn-primary {{
    background: #2563eb;
    color: #ffffff;
    border: none;
    border-radius: 8px;
    padding: 10px 20px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s ease;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }}
  .btn-primary:hover {{ background: #1d4ed8; }}
  .btn-secondary {{
    background: #1c1f28;
    color: #e2e8f0;
    border: 1px solid #2d3342;
    border-radius: 8px;
    padding: 10px 18px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }}
  .btn-secondary:hover {{ background: #282d3b; border-color: #3f475b; }}
  details.diagnostics {{
    border-top: 1px solid #1e222b;
    padding-top: 16px;
  }}
  summary {{
    font-size: 12px;
    color: #64748b;
    cursor: pointer;
    font-weight: 600;
    user-select: none;
  }}
  summary:hover {{ color: #94a3b8; }}
  .diag-box {{
    margin-top: 12px;
    background: #07080a;
    border: 1px solid #1a1d24;
    border-radius: 6px;
    padding: 12px;
    font-family: ui-monospace, "JetBrains Mono", monospace;
    font-size: 11px;
    color: #94a3b8;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-all;
  }}
</style>
</head>
<body>

<div class="error-card">
  <div class="top-row">
    <div class="badge-category">
      <span class="badge-dot"></span>
      <span>{category}</span>
    </div>
  </div>

  <div class="error-code">{code}</div>
  <h1 class="error-title">{title}</h1>
  <div class="error-uri">{safe_uri}</div>

  <p class="error-desc">{desc}</p>

  <div class="fixes-card">
    <div class="fixes-title">{fixtitle}</div>
    <ul class="fixes-list">
      {fixes_html}
    </ul>
  </div>

  <div class="actions-row">
    <button class="btn-primary" onclick="retryLoad()">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M21.5 2v6h-6M2.5 22v-6h6M2 11.5a10 10 0 0 1 18.8-4.3M22 12.5a10 10 0 0 1-18.8 4.2"/></svg>
      <span>{reload_lbl}</span>
    </button>
    <button class="btn-secondary" onclick="window.location.href='rail://start/'">{home_lbl}</button>
    <button class="btn-secondary" onclick="copyDiag()" id="t-copy">{copy_lbl}</button>
  </div>

  <details class="diagnostics">
    <summary>{diag_lbl}</summary>
    <div class="diag-box" id="diag-content">Error: {safe_raw}
Code: {code}
URL: {safe_uri}
Engine: WebKitGTK / Linux x86_64
Timestamp: {timestamp}</div>
  </details>
</div>

<script>
function retryLoad() {{
  window.location.href = {safe_uri_js};
}}

function copyDiag() {{
  var text = document.getElementById('diag-content').innerText;
  navigator.clipboard.writeText(text).then(function() {{
    var btn = document.getElementById('t-copy');
    var old = btn.textContent;
    btn.textContent = "{copied_lbl}";
    setTimeout(function() {{ btn.textContent = old; }}, 1500);
  }});
}}
</script>
</body>
</html>"##,
        code = detail.code,
        category = detail.category,
        badge_color = detail.badge_color,
        title = title,
        desc = desc,
        fixtitle = fixtitle,
        fixes_html = fixes_html,
        reload_lbl = reload_lbl,
        home_lbl = home_lbl,
        copy_lbl = copy_lbl,
        copied_lbl = copied_lbl,
        diag_lbl = diag_lbl,
        safe_uri = safe_uri,
        safe_uri_js = format!("\"{}\"", crate::minijson::escape(failing_uri)),
        safe_raw = safe_raw,
        timestamp = timestamp,
    )
}
