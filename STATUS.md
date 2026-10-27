# AI Context — .

## Oxirgi holat
- Sana: 2026-10-15 20:25
- Nima qilindi: DevTools undocked oyna yopilgandagi (Win+Q) crash tuzatildi, HTML Color Chooser modal dialogi o'rnatildi, Ctrl+1..9 tab almashtirish qo'shildi, End-to-End Encrypted Cloud Sync (Supabase + Vercel) ulandi. v0.5.5 release yaratildi va o'rnatildi.
- Hozirgi muammo/blocker: Yo'q.
- Keyingi qadam: Foydalanuvchi Rail'ni qayta ochsin.

## Muhim fayllar
- src/main.rs — Ctrl+1..9 tab switcher, DevTools lifecycle, run_color_chooser, Cloud Sync UI
- src/sync.rs — AES-256 Cloud Sync pack & download logic
- src/i18n_tables.rs — tarjimalar

## Eslatmalar
- Release: https://github.com/mrdevs2011/rail/releases/tag/v0.5.5
- SHA256: bffa516132776ba823430301351dfff494c98594e335fb9c3e25731944c94055

---
### 2026-10-19 18:00
- Qilindi: Uxlagan tabda kulrang placeholder (#121212) butunlay yoq qilindi.
  - .tab-freeze background transparent (CSS).
  - Freeze overlay faqat haqiqiy rasm bolsa korinadi; rasm yoq bolsa process baribir oldiriladi (URL saqlanadi).
  - Freeze frame endi max 480px kenglikka kichraytiriladi (~0.5-1 MB) — barcha uxlagan tablar rasmini saqlab qolish mumkin, prune yoq.
  - overview / focus wake xuddi shu qoida.
- Build: cargo build --release OK.
- Hali qilinmagan: Tab.view = Option (WebView widgetini ham toliq tashlash) — keyingi bosqich.
- Git: uncommitted (lifecycle, overview, focus, style.css).
---
### 2026-10-16 13:42
- Qilindi: v0.5.8 release: suspend (bo'sh tab) xatosi, omnibar inline to'ldirish xatosi tuzatildi, sidebar'dan TO'LIQ belgisi olib tashlandi, workspace tab soni va + hover'da
- Keyingi qadam: Foydalanuvchi Rail'ni qayta ochib tab almashtirish va omnibar to'ldirishni sinasin
- Git holati: 461222f fix(omnibar): inline to'ldirish har ikkinchi harfda ishlamasligi tuzatildi (v0.5.8)

---
### 2026-10-16 15:00
- Qilindi: v0.5.9 release: rasm paste (WebKitGTK bug 218519 aylanma yo'li), DevTools F12 stack overflow guard, DevTools alohida oyna floating emas (transient_for olib tashlandi), eskirgan koordinata ushlagichlar o'chirildi, Left/Right dock slot orqali
- Keyingi qadam: Foydalanuvchi Left/Right dock va Gemini paste ni real sinab ko'rsin; noto'g'ri bo'lsa logdagi [rail] inspector attach qatori bo'yicha mapping tuzatish; undock'dan dock hozir faqat Bottom
- Git holati: 18d6084 fix: rasm paste (WebKitGTK), DevTools F12 stack overflow, alohida oyna, Left/Right dock (v0.5.9)

---
### 2026-10-16 22:45
- Qilindi: CI to'liq — main push da avtomatik Linux build. release.yml tuzatildi (to'g'ri asset nomi + SHA256SUMS). index.html/package.json versiya 0.5.9.
- Git holati: CI workflows + index.html yangilandi

---
### 2026-10-17 06:19
- Qilindi: v0.5.10: klassik bir sahifalik index.html (faqat Linux), release + o'rnatish
- Keyingi qadam: Foydalanuvchi Rail ni qayta ochsin; sayt Vercel da yangilanishi mumkin
- Git holati: 032827f site: klassik bir sahifalik index.html, faqat Linux (v0.5.10)

---
### 2026-10-17 06:47
- Qilindi: v0.5.11: tab freeze 10m, uyg'onganda reload; release
- Keyingi qadam: Foydalanuvchi Rail ni qayta ochsin
- Git holati: ffa2b09 feat: tab freeze (10 daqiqa uxlash) + uyg'onganda yangilash (v0.5.11)

---
### 2026-10-17 06:54
- Qilindi: v0.5.12: smooth scroll/slide, GPU Always, frame pan, WebGL; release
- Keyingi qadam: Foydalanuvchi Rail ni qayta ochsin
- Git holati: 5bf89d2 fix: scroll/slide silliqligi — GPU, frame pan, WebGL (v0.5.12)

---
### 2026-10-17 07:07
- Qilindi: v0.5.13: folder URLs + smooth scroll + auto-update + SVG gear; release (install skip)
- Keyingi qadam: Deploy Vercel; user can install manually
- Git holati: b59d86e feat: folder URLs, smooth scroll, auto-update, SVG gear (v0.5.13)

---
### 2026-10-17 07:12
- Qilindi: v0.5.14: New version chip in settings header, filled SVG gear; release (no install)
- Keyingi qadam: User updates via Settings or card
- Git holati: 8837a4a feat: settings header New version chip + filled SVG gear (v0.5.14)

---
### 2026-10-18 22:56
- Qilindi: Animatsiya freeze-kadr (slide/overview) + F11 content-only fullscreen; TMPDBG olib tashlandi; 44 unit test OK; release build OK. CLAUDE.md qo'shildi.
- Hozirgi muammo/blocker: headless testda F11+Alt+H dan keyin session'da rail://downloads ko'rinishi — kod yo'li topilmadi, real GPU'da tekshirish kerak.
- Keyingi qadam: main ga push → auto-release v0.5.72
- Git holati: uncommitted → commit + push

---
### 2026-10-18 23:05
- Qilindi: RAM/exit fix — swap-aware suspend (max 3 live under pressure), tighter defaults (6/4/3), window close kills all WebKit + app.quit, focused reload rate-limit 4s, default WebKit MemoryPressure 768MB kill=0.
- Tahlil: "yopilgan" Rail aslida tirik edi (~108MB + web process). Tizim swap 4GB to'liq — lockscreen WebKit ham ~157MB yegan. Rail emas, umumiy OOM.
- Keyingi: push → auto-release v0.5.73

---
### 2026-10-19 14:41
- Qilindi: Sahifa tarjimasi sozlamalari to'liq: Settings > General > "Sahifa tarjimasi" (Asl til default auto, Nishon til default uz), config: translate_source_lang / translate_target_lang. Til ro'yxati SUPPORTED_LANGUAGES (145 ta, nav/translate.rs) dropdownlarga ulandi; "zh" Google uchun zh-CN ga o'giriladi. Kontekst menyu "Sahifani tarjima qilish" (Google Translate proxy) oldindan bor edi. Release build OK, 53 test OK.
- Hozirgi muammo/blocker: yo'q. Ba'zi kodlar (ab, aa, av, kr, kv, ...) Google Translate tomonidan qo'llanmasligi mumkin.
- Keyingi qadam: real oynada qo'lda tekshirish, keyin commit + push
- Git holati: uncommitted (config.rs, general.rs, nav/translate.rs, app/translate.rs, i18n_tables.rs, tests.rs ...)

---
### 2026-10-19 14:50
- Qilindi: Til dropdownlariga bayroqlar qo'shildi: ~/Downloads/flag-icons-main dan 102 ta bayroq 48x36 PNG qilib assets/flags/ ga (jami ~39 KB, LICENSE bilan), src/ui/flags.rs (til -> bayroq, taxminiy; auto va eo bayroqsiz), general.rs da bayroq+nom factory, CSS .flag-icon. Build OK, 54 test OK.
- Hozirgi muammo/blocker: yo'q. Real oynada ko'rinishi tekshirilmagan.
- Keyingi qadam: Rail ni ochib Settings > General > Sahifa tarjimasi ni ko'zdan kechirish, keyin commit + push
- Git holati: uncommitted

---
### 2026-10-19 14:57
- Qilindi: Tarjima qilingan (*.translate.goog) sahifada ham kontekst menyuda "Sahifani tarjima qilish" chiqadi (asl URL Settings tillari bilan qayta tarjima qilinadi). Release build OK.
- Hozirgi muammo/blocker: yo'q
- Keyingi qadam: yangi release o'rnatilgach real sahifada tekshirish
- Git holati: commit + push

---
### 2026-10-19 15:01
- Qilindi: Til dropdownida bayroq o'lchami tuzatildi (gtk::Image pixel_size 24, tugma va ro'yxatda bir xil); "Avto aniqlash" uchun globus SVG ikonka. Release build OK.
- Hozirgi muammo/blocker: yo'q
- Keyingi qadam: yangi release o'rnatilgach real oynada tekshirish
- Git holati: commit + push
### 2026-10-19 12:59
- Qilindi: Omnibar takliflari limiti 5 -> 20, scrollbar doim ko`rinadi (overlay o`chirildi), klaviatura bilan tanlaganda ro`yxat qayta chizilmaydi (scroll joyida qoladi, tanlangan qator ko`rinishga o`tadi). Sayt ruxsat so`rovi oynasi qo`shildi (mikrofon/kamera/joylashuv/bildirishnoma: "Allow while visiting the site" / "Allow this time" / "Never allow"), enable_media_stream yoqildi, i18n uz/en/ru.
- Hozirgi muammo/blocker: yo`q. cargo build --release OK; cargo test: 43 OK, 1 FAIL (update::test_fetch_releases_gets_more_than_ten, tarmoqqa bog`liq, bu o`zgarishga aloqasi yo`q).
- Keyingi qadam: real oynada qo`lda tekshirish (claude.ai mikrofon so`rovi, uzun tarixli omnibar). "Allow while visiting" / "Never allow" hozircha faqat ilova sessiyasi davomida eslab qolinadi (diskka saqlanmaydi).
- Git holati: uncommitted (src/app/dialogs.rs, palette.rs, signals.rs, tabs.rs, history.rs, i18n_tables.rs, ui/widgets/palette.rs)
---
### 2026-10-19 16:30
- Qilindi: Context menu header (Back/Forward/Reload bir qatorda) + Zen-uslubidagi Take Screenshot:
  element hover outline (JS), click-to-scope, resizable region, auto-scroll pastga/tepaga,
  Save visible / Save full page, Copy / Download. WebKit snapshot API.
  Fayllar: src/app/screenshot.rs (yangi), ctxmenu.rs, style.css, i18n_tables.rs, mod.rs.
- Build: cargo build --release OK (faqat warning).
- Keyingi qadam: Foydalanuvchi sahifada o'ng bosib "Skrinshot olish" ni sinasin; overlay joylashuvi
  webview ustida to'g'ri tushmasa parent/overlay ni sozlash kerak bo'lishi mumkin.
---
### 2026-10-19 16:55
- Qilindi: Uxlagan tab hech qachon qop-qora bo'lmasin.
  - Oxirgi kadr MemoryTexture (durable) ga ko'chiriladi — web process o'lgach ham o'chmaydi.
  - Bo'sh/qora kadr rad etiladi, oldingi yaxshi frame saqlanadi.
  - Suspend paytida freeze overlay har doim yoqiladi; frame yo'q bo'lsa #121212 placeholder.
  - prune_freeze_frames: xotira holatiga qarab max 3/5/10 ta texture, eskilar olib tashlanadi.
  - Visibility hide, overview, anim, focus wake pathlari yangilandi.
- Build: cargo build --release OK.
- Keyingi: foydalanuvchi bir necha tab ochib uxlatib ko'rsin.

---
### 2026-10-19 17:47
- Qilindi: Uxlagan tab / overview kartasida kulrang placeholder (#121212) chiqmasligi uchun: rasmi yo'q tab endi uxlatilmaydi (suspend_tab va overview.rs, tirik qoladi); prune_freeze_frames olib tashlandi (uxlagan tab rasmi hech qachon o'chirilmaydi); rail:// sahifalarining qora kadri endi rad etilmaydi (durableize_paintable allow_black).
- Build: cargo build --release OK; cargo test --release 57 OK.
- Hozirgi muammo/blocker: sessiyadan tiklangan (lazy) tablarda bir marta ochilmaguncha rasm yo'q, ular hali kulrang chiqishi mumkin. Rasmlar kichraytirilmaydi, tab ko'p bo'lsa RAM'da ko'proq joy oladi.
- Keyingi qadam: Alt+O bilan real oynada tekshirish. "Uxlaganda WebView to'liq yopilsin (faqat rasm + URL qolsin)" hali qilinmagan: Tab.view ni Option qilish kerak.
- Git holati: uncommitted (src/app/lifecycle.rs, src/app/overview.rs)

---
### 2026-10-21 13:50
- Qilindi: Overview zoom-in endi markaziy nuqta atrofida sof zoom (scroll.rs animate): oxirida tepaga surilish/oynab qaytish yo`q. Release build OK.
- Blocker: video (YouTube iframe) ishlamasligi sababi tizimda gst-libav / gst-plugins-bad yo`qligi (avdec_h264, avdec_aac). Drag-drop, paste, upload sababi hali aniqlanmagan.
- Keyingi qadam: real oynada zoom-in ni tekshirish; drag-drop/upload/paste ni qayta ishlab chiqarib sababini topish.

---
### 2026-10-23
- Qilindi: kvadrat ikonka tugmalar (btn-add-tab, flat-icon, tab-close, btn-ws-add), profil doira/nuqta/avatar (profile-circle-btn, profile-item-dot, profile-avatar-circle), color-choice va profile-badge-mini endi doira/kapsula (style.css oxiriga override blok). Panel, qator, modal va tab kartalari o`zgarmadi (to`rtburchak).
- Build: cargo build --release OK.
- Keyingi qadam: real oynada ko`zdan kechirish (tab-close/hover holatlari).

---
### 2026-10-27
- Qilindi: DevTools ochish/yopish uchun `Ctrl+Shift+I` kombinatsiyasi qo'shildi (`F12` bilan birga). `src/shortcuts/config.rs` va `docs/shortcuts/index.html` yangilandi.
- Build: cargo build --release OK (GUI o'tkazilmadi — xvfb mavjud emas, Ctrl+Shift+I yo'li kod orqali tekshirildi).
