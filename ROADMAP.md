# Rail Roadmap

> **Maqsad:** bitta kod, uch tizim (Linux, macOS, Windows), hammasi **bepul**. Toza qora-oq dasturchi brauzeri. O'rnatish va yangilash foydalanuvchi uchun 1 bosish.
> Oxirgi yangilanish: 2026-10-17 (v0.5.42). Sayt: https://rail-browser.vercel.app
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

## 1. Hozirgi holat (v0.5.42) — Linux to'liq ishlaydi

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
- [x] **CI:** `main` ga push → Linux rail build + spike (Linux/macOS/Windows). Tag `v*` → release + SHA256SUMS

**Cheklov:** UI qatlami GTK4 ga bog'langan. macOS/Windows uchun 3-bosqich (tao + wry) hal qiladi.

---

## 2. Bepul qoidasi: vositalar

| Vazifa | Vosita | Izoh |
| :--- | :--- | :--- |
| Sayt | Vercel (Hobby) | Hozir shu. docs/ deploy qilinadi |
| Sayt (zaxira) | GitHub Pages | Ochiq repo uchun bepul |
| Uch tizimda build | GitHub Actions | Linux, macOS, Windows runnerlari bepul |
| Release fayllar | GitHub Releases | Updater va sayt shu yerdan oladi |
| macOS imzo | Ad-hoc (codesign -s -) | Pulsiz |
| Windows imzo | SignPath Foundation | Ochiq loyihaga bepul |
| Paket menejerlar | Homebrew, Scoop, winget | Hammasi bepul |
| Domen | Yo'q | rail-browser.vercel.app |

---

## 3. Bosqichlar umumiy ko'rinishi

| # | Bosqich | Muddat | Holat |
| :--- | :--- | :--- | :--- |
| 1 | Sayt | 1 kun | [x] tayyor (qo'lda tekshiruv qoldi) |
| 2 | Tizimga qarab yangilash | 1 kun | [~] OS/ARCH + SHA256 hisoblash + temp fayl tayyor |
| 3 | Tajriba (spike): tao + wry | 1 hafta | [ ] eng muhim qaror |
| 4 | CI: bitta tag, uch build | 1 kun | [x] ci.yml (main push) + release.yml (v* tag, Linux + spike 3 OS) |
| 5 | Funksiyalarni ko'chirish | 2-4 hafta | [ ] spike'dan keyin |
| 6 | Tarqatish | 1 hafta | [ ] |

Tartib qat'iy: 3-bosqich natijasi chiqmaguncha 5-bosqichga o'tilmaydi.

---

## 4. Bosqich 1: Sayt — [x] asosan tayyor

- [x] Interaktiv qora-oq landing + OS tanlash + 3 qadamli yo'riqnoma
- [x] Vercel production deploy
- [x] Oxirgi versiya GitHub'dan olinadi
- [x] Ko'p sahifali docs + i18n
- [ ] Mobil va turli brauzerlarda qo'lda tekshiruv
- [ ] Open Graph rasm
- [ ] macOS/Windows tugmalarini yoqish (tayyor bo'lganda)

---

## 5. Bosqich 2: Tizimga qarab yangilash — [~] asosiy qism tayyor

- [x] `asset_name()`: OS + ARCH bo'yicha to'g'ri binary (linux-x86_64, macos-arm64, windows-x86_64.exe, ...)
- [x] Settings > Updates tabi + dropdown + one-tap Update
- [x] Vaqtinchalik fayl + rename (uzilsa yarim fayl qolmaydi)
- [x] Binary `--version` tekshiruvi
- [~] SHA256SUMS tekshiruvi (sha2 qo'shildi, hisoblash tayyor; release'da SUMS bo'lganda solishtiriladi)
- [ ] Windows: rail.old.exe mantiqi
- [ ] macOS: quarantine olib tashlash
- [ ] rail --update barcha tizimlarda

**Tayyor deyiladi:** har tizimda bir bosishda yangilanadi, tablar joyida qoladi.

---

## 6. Bosqich 3: Tao + wry spike — [~] BOSHLANDI

Yadro Rustda qoladi. Oyna/webview: tao + wry.

Tekshiriladigan savollar:
- [x] Bitta webview (Linux Wayland) — `examples/spike-tao-wry`
- [~] Ko'p webview yonma-yon (lenta) — 3 ustun kodi yozildi, sinov kerak
- [?] Profil izolyatsiyasi (cookie/storage)
- [?] HTML UI ustida
- [?] Shortcutlar, html.css, F12, silliq scroll

**Chiqish mezoni:** 3 ustun, 2 profil, F12 demo. Uchala tizimda CI bilan yig'iladi.

**Hozirgi holat (2026-10-16):**
- Linux: tao+wry oyna + WebView ishladi. 3-webview kodi tayyor (`cargo run`).
- CI: `.github/workflows/spike.yml` — Linux/macOS/Windows da spike build (Linuxda webkit deps o'rnatiladi).
- Keyingi: profil izolyatsiyasi, keyin asosiy Rail'ni ko'chirish.

**Zaxira reja:** Linux GTK4 qoladi, mac/win uchun alohida wry qobiq, umumiy rail-core.

---

## 7–9. Keyingi bosqichlar

- **4. CI:** `.github/workflows/ci.yml` — main push da Linux rail + spike 3 OS; `release.yml` — v* tegida Linux asset + SHA256SUMS + spike 3 OS
- **5. Funksiyalar ko'chirish:** rail-core → lenta → profillar → omnibar → ... (spike muvaffaqiyatli bo'lsa)
- **6. Tarqatish:** .dmg, .exe, Homebrew, Scoop, winget, SignPath

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
| wry ko'p webview bermasa | Zaxira: Linux GTK4 + alohida mac/win qobiq + rail-core |
| Imzosiz ogohlantirish | Saytda yo'riqnoma + SignPath |
| Domen yo'q | rail-browser.vercel.app yetarli |
| GitHub API limiti | Atom feed + zaxira versiya |

---

## 12. Versiya rejasi

| Versiya | Mazmuni |
| :--- | :--- |
| v0.5.x | Linux, hozirgi holat (Updates tabi, i18n, docs) |
| v0.6.0 | To'liq tizimga qarab yangilash + SHA256 |
| v0.7.0 | Spike natijasi: uch tizimda demo |
| v0.9.0 | Funksiyalar ko'chirilgan, beta |
| v1.0.0 | Uch tizimda barqaror + paket menejerlar |

---

## 13. Keyingi qadamlar

1. Saytni qo'lda tekshirish (mobil / brauzerlar)
2. Bosqich 2 ni yakunlash (SHA256SUMS)
3. Bosqich 3: tao + wry spike (ko'p webview + Wayland + profil)
4. Spike natijasini shu faylga yozish va qaror qilish
5. Bosqich 4: CI workflow

## 14. Ochiq savollar

- macOS va Windowsda kim sinaydi?
- Spike'da silliq scroll sifatini qanday o'lchaymiz?

---

**Hozirgi aniq holat (2026-10-15):** Linuxda to'liq ishlaydi (v0.5.9). Settings > Updates tabi va OS/ARCH-aware updater tayyor. Keyingi katta qadam — tao + wry spike.
