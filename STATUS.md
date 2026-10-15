# AI Context — .

## Oxirgi holat
- Sana: 2026-10-15 18:30
- Nima qilindi: Settings > Updates tabi qo‘shildi (joriy versiya, status, GitHub releases dropdown, one-tap Update). Updater endi OS/ARCH bo‘yicha asset tanlaydi (asset_name()). ROADMAP Bosqich 2 progress belgilandi. Commit: bfd2cc0
- Hozirgi muammo/blocker: SHA256SUMS hali release’da yo‘q (hozir --version tekshiruvi yetarli). Bosqich 3 (tao+wry spike) keyin.
- Keyingi qadam: Saytni qo‘lda tekshirish yoki Bosqich 3 spike boshlash.

## Muhim fayllar
- src/update.rs — asset_name(), fetch_releases(), download_and_install
- src/main.rs — SettingsTab::Update, updates page
- ROADMAP.md — Bosqich 2 yangilandi

## Eslatmalar
- 100% bepul yo‘l saqlanadi. Domen yo‘q, Vercel + GitHub Releases.
- Hozirgi versiya v0.5.2 (eng so‘nggi).
