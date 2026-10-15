# AI Context — .

## Oxirgi holat
- Sana:
- Nima qilindi:
- Hozirgi muammo/blocker:
- Keyingi qadam:

## Muhim fayllar
-

## Eslatmalar (arxitektura, qarorlar, "buni qilma" kabi)
-

---
### 2026-10-15 16:39
- Qilindi: docs/index.html (interaktiv ribbon sayt) yaratildi va Vercel'ga deploy qilindi: rail-browser.vercel.app. ROADMAP.md to'liq qayta yozildi.
- Keyingi qadam: rail.uz domen ulash, 2-bosqich updater (OS/ARCH + SHA256), git commit/push hali qilinmagan
- Git holati: 0ff5c40 docs: add ROADMAP.md (cross-platform, free path)

---
### 2026-10-15 16:43
- Qilindi: rail-browser: main'ga push (5b182d7), v0.5.1 release sha mos, kompyuterda v0.5.1 o'rnatildi, install.sh dublikat xatolari tuzatildi, ROADMAP: domen olinmaydi + 100% bepul
- Keyingi qadam: eski root-egali ~/Claude/work/rail (1.8GB) sudo bilan o'chirish, ishlayotgan Rail'ni qayta ochish
- Git holati: 5b182d7 site: interactive landing (docs/index.html), roadmap rewrite (free path, no domain), installer: atomic update + single desktop entry

---
### 2026-10-15 16:50
- Qilindi: Eski ~/Claude/work/rail (v0.1.0, 1.8GB) o'chirildi; tekshiruv: work da faqat rail-browser, which -a rail faqat ~/.local/bin/rail (sha ba4ad6da... mos), bitta rail.desktop, git main...origin/main toza, sayt 200
- Keyingi qadam: Rail brauzer va docs/index.html ni to'liq o'zbekcha qilish (uz default, uz/en/ru toggle, Settings > Language)
- Git holati: 5b182d7 site: interactive landing (docs/index.html), roadmap rewrite (free path, no domain), installer: atomic update + single desktop entry

---
---
### 2026-10-15 18:05
- Qilindi: 
  1. index.html root'ga ko'chirildi va Hujjatlar (docs/) havolalari qo'shildi.
  2. docs/ papkasi to'liq ko'p faylli (multi-page) hujjatlar tizimiga aylantirildi: index.html, architecture.html, shortcuts.html, profiles.html, installation.html, configuration.html, docs.css, docs.js, i18n.js.
  3. Desktop leftbar (sidebar) va mobil hamburger menyu drawer tizimi yaratildi.
  4. Interaktiv virtual komponentlar: 2D Virtual Ribbon Simulyatori, Profil Izolyatsiya Laboratoriyasi, Klaviatura Trenajyori.
  5. 3 tilda (UZ, EN, RU - sukut bo'yicha UZ) to'liq mahalliylashtirish va root bilan docs o'rtasida real-time ikki tomonlama sinxronizatsiya (BroadcastChannel + localStorage + storage listener).
- Git holati: yangi fayllar yaratildi, test qilindi.

