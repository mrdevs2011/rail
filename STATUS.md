# AI Context — .

## Oxirgi holat
- Sana: 2026-10-15 18:35
- Nima qilindi: SHA256 hisoblash (sha2) qo‘shildi. CI release workflow skeleton (.github/workflows/release.yml) yaratildi (Linux/macOS/Windows). ROADMAP yangilandi. Binary qayta o‘rnatildi.
- Hozirgi muammo/blocker: To‘liq macOS/Windows test va tao+wry spike bu muhitda amalga oshirilmaydi (faqat Linux). SHA256SUMS hali release’da yo‘q.
- Keyingi qadam: git push, keyin Bosqich 3 spike (tao+wry) yoki sayt tekshiruvi.

## Muhim fayllar
- src/update.rs — asset_name(), SHA256, fetch_releases
- .github/workflows/release.yml — multi-platform release
- ROADMAP.md — yangilangan progress

## Eslatmalar
- 100% bepul yo‘l saqlanadi.
- Hozirgi versiya: v0.5.2
