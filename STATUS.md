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
- Qilindi: CI to'liq — main push da avtomatik 3 OS build (Rail Linux + spike Linux/macOS/Windows). release.yml tuzatildi (to'g'ri asset nomi + SHA256SUMS). index.html/package.json versiya 0.5.9. Mac/Win asosiy binary hali GTK4 sabab chiqmaydi — spike orqali yo'l ochiq.
- Keyingi qadam: tao+wry spike da ko'p webview + profil izolyatsiyasini yakunlash, keyin asosiy Rail ni ko'chirish.
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
