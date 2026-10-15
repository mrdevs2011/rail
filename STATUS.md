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
### 2026-10-15 17:35
- Qilindi: v0.5.2: uz/en/ru i18n (default uz), Settings>Interface Language, sayt 3 tilda + header toggle, Production.md, release.sh; push+release+o'rnatish bajarildi
- Keyingi qadam: Windows/macOS build, Vercel deploy (kirish yo'q), tilni qayta ochmasdan almashtirish
- Git holati: 09468e5 feat(i18n): uz/en/ru tizimi (default uz), Settings > Interface Language, to'liq o'zbekcha sayt + header toggle, Production.md, release.sh (v0.5.2)
