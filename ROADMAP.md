# Rail Roadmap

> **Maqsad:** faqat Linux, hammasi **bepul**. Toza qora-oq dasturchi brauzeri. O'rnatish va yangilash foydalanuvchi uchun 1 bosish.
> Oxirgi yangilanish: 2026-10-17 (v0.5.113). Sayt: https://rail-browser.vercel.app
>
> **Qaror:** rail.uz domeni olinmaydi. Sayt Vercel'da (rail-browser.vercel.app) turaveradi. Butun yo'l 100% bepul.

Belgilar: [x] tayyor, [ ] qilinadi, [?] tajribada tekshiriladi, [~] boshlangan / qisman.

---

## 0. Tamoyillar (o'zgarmaydi)

1. **100% bepul yo'l.** Hech narsaga pul to'lanmaydi: hosting, imzo, domen. Pulli variantlar yo'q.
2. **Qora-oq.** Rang faqat bitta: ko'k nuqta. Yangi UI shu qoidaga bo'ysunadi.
3. **Klaviatura birinchi.** Har amalga bitta klavish. Sayt tugmalari bilan to'qnashmaydi.
4. **Profil = alohida dunyo.** Cookie, storage, kesh aralashmaydi.
5. **Yadro bitta.** Rust yadro umumiy, faqat oyna/webview qatlami tizimga qarab almashadi.
6. **Sodda yo'l birinchi.** Har qadamda eng kichik ishlaydigan narsa chiqadi, keyin silliqlanadi.

---

## 1. Hozirgi holat (v0.5.113) — Linux to'liq ishlaydi

- [x] Rasm paste (Ctrl+V va o'ng tugma): WebKitGTK DataTransfer'da rasm bermaydi (WebKit bug 218519), Rail rasmni o'zi o'qib sahifaga paste hodisasi sifatida beradi (Gemini va h.k.)
- [x] DevTools: F12 da stack overflow (SIGABRT) tuzatildi, alohida oyna floating bo'lmay oddiy oyna bo'lib ochiladi, Left/Right dock tugmalari va eskirgan koordinata ushlagichlari tuzatildi
- [x] End-to-end shifrlangan Cloud Sync (Vercel + Supabase, majburiy parol, zero-knowledge)
- [x] Soddalashtirilgan, o'ta toza Cloud Sync & Data sozlamalar sahifasi
- [x] Linux: GTK4 + WebKitGTK, niri uslubidagi 2D lenta (ustunlar + vertikal stack)
- [x] Izolyatsiyalangan profillar, profil belgisi (3 xonagacha)
- [x] Qora-oq dev UI, ko'k faqat nuqta
- [x] Barcha ichki shortcutlar Alt orqali
- [x] Tahrirlanadigan html.css (Sozlamalar > Customize)
- [x] **Settings > Updates tabi:** joriy versiya, status, GitHub releases dropdown, one-tap Update
- [x] Yangilanish kartasi + atom feed (API limiti yo'q)
- [x] OS/ARCH-aware asset tanlash (`asset_name()`)
- [x] rail --update terminaldan
- [x] Ikonka: qora kvadrat, oq r, ko'k nuqta
- [x] install.sh + AppImage skripti
- [x] To'liq i18n: o'zbekcha (default), English, Русский — Settings > Interface Language + sayt
- [x] Ko'p sahifali docs/ (architecture, shortcuts, profiles, installation, configuration) + real-time i18n sync
- [x] **Sayt jonli:** https://rail-browser.vercel.app
- [x] **CI:** `main` ga push → Linux rail build. Tag `v*` → release + SHA256SUMS


---

## 2. Bepul qoidasi: vositalar

| Vazifa | Vosita | Izoh |
| :--- | :--- | :--- |
| Sayt | Vercel (Hobby) | Hozir shu. docs/ deploy qilinadi |
| Sayt (zaxira) | GitHub Pages | Ochiq repo uchun bepul |
| Release fayllar | GitHub Releases | Updater va sayt shu yerdan oladi |
| Paket menejerlar | Homebrew, Scoop, winget | Hammasi bepul |
| Domen | Yo'q | rail-browser.vercel.app |

---

## 3. Bosqichlar umumiy ko'rinishi

| # | Bosqich | Muddat | Holat |
| :--- | :--- | :--- | :--- |
| 1 | Sayt | 1 kun | [x] tayyor (qo'lda tekshiruv qoldi) |
| 2 | Tizimga qarab yangilash | 1 kun | [~] OS/ARCH + SHA256 hisoblash + temp fayl tayyor |
| 4 | CI: Linux build va release | 1 kun | [x] ci.yml (main push) + release.yml (v* tag, Linux) |
| 6 | Tarqatish | 1 hafta | [ ] |


---

## 4. Bosqich 1: Sayt — [x] asosan tayyor

- [x] Interaktiv qora-oq landing + OS tanlash + 3 qadamli yo'riqnoma
- [x] Vercel production deploy
- [x] Oxirgi versiya GitHub'dan olinadi
- [x] Ko'p sahifali docs + i18n
- [ ] Mobil va turli brauzerlarda qo'lda tekshiruv
- [ ] Open Graph rasm

---

## 5. Bosqich 2: Tizimga qarab yangilash — [~] asosiy qism tayyor

- [x] `asset_name()`: OS + ARCH bo'yicha to'g'ri binary (linux-x86_64, linux-aarch64)
- [x] Settings > Updates tabi + dropdown + one-tap Update
- [x] Vaqtinchalik fayl + rename (uzilsa yarim fayl qolmaydi)
- [x] Binary `--version` tekshiruvi
- [~] SHA256SUMS tekshiruvi (sha2 qo'shildi, hisoblash tayyor; release'da SUMS bo'lganda solishtiriladi)
- [ ] rail --update barcha tizimlarda

**Tayyor deyiladi:** har tizimda bir bosishda yangilanadi, tablar joyida qoladi.

---

## 7–9. Keyingi bosqichlar

- **4. CI:** `.github/workflows/ci.yml` — main push da Linux rail build; `release.yml` — v* tegida Linux asset + SHA256SUMS
- **6. Tarqatish:** AppImage va Linux paket menejerlar

---

## 10. Qo'lda sinov ro'yxati (har release)

- [ ] Ochiladi, ikonka, "Rail"
- [ ] Yangi tab, qidiruv, ikki profil (cookie aralashmaydi)
- [ ] Sessiyani tiklash, shortcutlar, F12
- [ ] Update + rail --update
- [ ] Kichik/yarim ekran UI
- [ ] Sayt yuklash tugmasi oxirgi release'ga olib boradi

---

## 11. Xavflar va choralar

| Xavf | Chorasi |
| :--- | :--- |
| Imzosiz ogohlantirish | Saytda yo'riqnoma + SignPath |
| Domen yo'q | rail-browser.vercel.app yetarli |
| GitHub API limiti | Atom feed + zaxira versiya |

---

## 12. Versiya rejasi

| Versiya | Mazmuni |
| :--- | :--- |
| v0.5.x | Linux, hozirgi holat (Updates tabi, i18n, docs) |
| v0.6.0 | To'liq tizimga qarab yangilash + SHA256 |
| v0.9.0 | Funksiyalar ko'chirilgan, beta |
| v1.0.0 | Linuxda barqaror + paket menejerlar |

---

## 13. Keyingi qadamlar

1. Saytni qo'lda tekshirish (mobil / brauzerlar)
2. Bosqich 2 ni yakunlash (SHA256SUMS)
3. Bosqich 4: CI workflow (Linux)

## 14. Ochiq savollar

- Hozircha yo'q.

---

**Hozirgi aniq holat (2026-10-15):** Linuxda to'liq ishlaydi (v0.5.9). Settings > Updates tabi va OS/ARCH-aware updater tayyor. Faqat Linux qo'llab-quvvatlanadi.
