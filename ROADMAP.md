# Rail Roadmap

> **Maqsad:** bitta kod, uch tizim (Linux, macOS, Windows), hammasi **bepul**. Toza qora-oq dasturchi brauzeri. O'rnatish va yangilash foydalanuvchi uchun 1 bosish.
> Oxirgi yangilanish: 2026-10-15. Sayt: https://rail-browser.vercel.app
>
> **Qaror (2026-10-15):** rail.uz domenini hozircha olmaymiz. Sayt Vercel'da (rail-browser.vercel.app) turaveradi. Butun yo'l 100% bepul.

Belgilar: [x] tayyor, [ ] qilinadi, [?] tajribada tekshiriladi, [~] boshlangan.

---

## 0. Tamoyillar (o'zgarmaydi)

1. **100% bepul yo'l.** Hech narsaga pul to'lanmaydi: hosting ham, imzo ham, domen ham. Pulli variantlar rejada yo'q.
2. **Qora-oq.** Rang faqat bitta: ko'k nuqta. Yangi UI shu qoidaga bo'ysunadi.
3. **Klaviatura birinchi.** Har amalga bitta klavish. Sayt tugmalari bilan to'qnashmaydi.
4. **Profil = alohida dunyo.** Cookie, storage, kesh aralashmaydi.
5. **Yadro bitta.** Rust yadro umumiy, faqat oyna/webview qatlami tizimga qarab almashadi.
6. **Sodda yo'l birinchi.** Har qadamda eng kichik ishlaydigan narsa chiqadi, keyin silliqlanadi.

---

## 1. Hozirgi holat (v0.5.1)

- [x] Linux: GTK4 + WebKitGTK, niri uslubidagi 2D lenta (ustunlar + vertikal stack)
- [x] Izolyatsiyalangan profillar, profil belgisi (3 xonagacha)
- [x] Qora-oq dev UI, ko'k faqat nuqta
- [x] Barcha ichki shortcutlar Alt orqali (saytlar bilan to'qnashmaslik uchun)
- [x] Tahrirlanadigan html.css (Sozlamalar > Customize)
- [x] Yangilanish kartasi: "N versiya orqada", Update bir bosishda yuklaydi va qayta ochadi
- [x] Yangilanish tekshiruvi GitHub atom fidi orqali (API limiti yo'q)
- [x] rail --update terminaldan
- [x] Ikonka: qora kvadrat, oq r, ko'k nuqta
- [x] install.sh (bir qatorli o'rnatish) va AppImage skripti (packaging/)
- [x] **Sayt jonli:** https://rail-browser.vercel.app (docs/index.html, Vercel)

**Cheklov:** UI qatlami GTK4 ga bog'langan. macOS/Windowsda GTK4 + WebKitGTK ishonchsiz, shuning uchun 3-bosqichdagi tajriba hal qiladi.

---

## 2. Bepul qoidasi: vositalar

| Vazifa | Vosita | Izoh |
| :--- | :--- | :--- |
| Sayt | Vercel (Hobby) | Hozir shu. docs/ papkasi deploy qilinadi. Hobby tarifi tijoriy bo'lmagan loyiha uchun |
| Sayt (zaxira) | GitHub Pages | Ochiq repo uchun bepul. Vercel muammo qilsa shunga o'tamiz |
| Uch tizimda build | GitHub Actions | Ochiq repo uchun Linux, macOS, Windows runnerlari bepul |
| Release fayllar | GitHub Releases | Updater va sayt shu yerdan oladi |
| macOS imzo | Ad-hoc (codesign -s -) | Pulsiz. Birinchi ochishda "Open Anyway" kerak |
| Windows imzo | SignPath Foundation | Ochiq loyihaga bepul. Ariza berilguncha imzosiz, SmartScreen "More info > Run anyway" |
| Paket menejerlar | Homebrew tap, Scoop bucket, winget | Hammasi bepul |
| Domen | Yo'q | rail.uz olinmaydi. Sayt rail-browser.vercel.app da qoladi. Domen rejada yo'q |

Pulli narsalar (Apple Developer $99/yil, Windows kod imzo sertifikati) **keyinroq va ixtiyoriy**. Ularsiz ham hammasi ishlaydi.

---

## 3. Bosqichlar umumiy ko'rinishi

| # | Bosqich | Muddat | Holat |
| :--- | :--- | :--- | :--- |
| 1 | Sayt (index.html) | 1 kun | [x] tayyor (Vercel), qo'lda tekshiruv qoldi |
| 2 | Tizimga qarab yangilash | 1 kun | [ ] |
| 3 | Tajriba (spike): tao + wry | 1 hafta | [ ] hal qiluvchi |
| 4 | CI: bitta tag, uch build | 1 kun | [ ] |
| 5 | Funksiyalarni ko'chirish | 2-4 hafta | [ ] spike'dan keyin |
| 6 | Tarqatish | 1 hafta | [ ] |

Tartib qat'iy: 3-bosqich natijasi chiqmaguncha 5-bosqichga o'tilmaydi.

---

## 4. Bosqich 1: Sayt (rail-browser.vercel.app)

Maqsad: 5 yoshli bola ham yuklab olib ishga tushira olsin.

- [x] docs/index.html: bitta fayl, kutubxonasiz, qora-oq, monospace
- [x] Sarlavha: **Welcome to rail**
- [x] Select your OS: brauzerdan OS ni o'zi tanlaydi (Linux, macOS, Windows)
- [x] Linux: katta Download tugmasi, bir qatorli o'rnatish buyrug'i (copy tugmasi bilan), 3 qadamli yo'riqnoma
- [x] macOS va Windows: "coming soon"
- [x] Interaktiv: sayt o'zi lenta (ustunlar chapga-o'ngga, drag, swipe, strelka, 0-4 raqamlar, minimap)
- [x] Profil izolyatsiyasi demosi, mini shell (help, version, update, roadmap), roadmap kartalari
- [x] Oxirgi versiya GitHub Releases API orqali olinadi (ko'rinadigan joyga yoziladi, qo'lda emas). Olinmasa v0.5.1 ko'rinadi
- [x] Vercel'ga production deploy (loyiha: rail-browser)
- [ ] Mobil va turli brauzerlarda qo'lda tekshiruv
- [ ] Open Graph rasm (havola ulashganda chiroyli chiqishi uchun)
- [x] Qaror: rail.uz olinmaydi, asosiy manzil rail-browser.vercel.app (bepul)
- [ ] macOS/Windows tayyor bo'lganda sayt tugmalarini yoqish

**Tayyor deyiladi:** telefondan va kompyuterdan ochiladi, OS tanlash 2 bosishda yuklashga olib boradi, yo'riqnoma 3 qadamdan oshmaydi.

---

## 5. Bosqich 2: Tizimga qarab yangilash (1 kun)

Hozir updater faqat rail-linux-x86_64 ni biladi.

- [ ] Release asset nomlari: rail-linux-x86_64, rail-macos-arm64, rail-macos-x86_64, rail-windows-x86_64.exe
- [ ] Koddagi std::env::consts::OS va ARCH orqali o'z tizimining faylini tanlash
- [ ] Yangilanish kartasi uchala tizimda bir xil: Update, yuklaydi, qayta ochadi
- [ ] Windows: ishlab turgan .exe ni almashtirib bo'lmaydi. Eskisini rail.old.exe ga o'zgartirib, yangisini joyiga qo'yadi, keyingi ochilishda rail.old.exe ni o'chiradi
- [ ] macOS: yuklangan faylga quarantine belgisini olib tashlash va bajarish ruxsati
- [ ] Yuklangan fayl SHA256 bilan tekshiriladi (release ichida SHA256SUMS). Mos kelmasa o'rnatilmaydi
- [ ] Yuklash uzilsa yarim fayl qolmaydi (vaqtinchalik nomga yozib, tugagach almashtirish)
- [ ] rail --update uchala tizimda

**Tayyor deyiladi:** har tizimda eski versiyadan yangisiga bir bosishda o'tadi, tablar joyida qoladi.

---

## 6. Bosqich 3: Tajriba (spike), 1 hafta. ENG MUHIM QAROR

Shu tajriba chiqmasa, qolgan ishni boshlamaymiz.

**Reja:** brauzer yadrosi Rustda qoladi. Oyna va webview uchun tao + wry (Linuxda WebKitGTK, macOSda WKWebView, Windowsda WebView2).

Tekshiriladigan savollar:

- [?] Bitta oynada **bir nechta webview** yonma-yon (lenta ustunlari) uchala tizimda ishlaydimi
- [?] Linux Waylandda child webview ishlaydimi (wry GTK konteyneri orqali)
- [?] Har profilga **alohida cookie va storage**
  - Windows: har profilga alohida WebView2 user data folder
  - macOS: WKWebsiteDataStore identifikatori (macOS 14+, eskisida cheklov)
  - Linux: alohida data directory
- [?] Webview ustida chiziladigan UI (sidebar, palette, update kartasi) HTML sifatida
- [?] Shortcutlar webview ichida ham ushlanadimi, sayt tugmalari bilan to'qnashmasdan
- [?] User stylesheet (html.css) va devtools (F12) har uch tizimda
- [?] Silliq scroll va touchpad bilan lenta surish (hozirgi Linux sifatiga yetadimi)
- [?] Anti-bot (stealth) sozlamalari wry da qayta qo'llanadimi

**Chiqish mezoni:** kichik demo: 3 ustun, 2 profil, shortcut bilan ustun almashtirish, F12. Uchala tizimda CI bilan yig'iladi va ishlaydi.

**Zaxira reja:** Linux GTK4 versiyasi shunday qoladi. macOS va Windows uchun alohida soddaroq wry qobig'i yoziladi. Yadro (profillar, sessiya, yangilash, sozlamalar) umumiy kutubxona rail-core bo'ladi.

**Qaror qoidasi:** birinchi 3 savol (ko'p webview, Wayland, profil izolyatsiyasi) uchala tizimda ishlasa, asosiy reja. Birortasi buzilsa, zaxira reja.

---

## 7. Bosqich 4: CI, bitta kod uch build (1 kun)

- [ ] .github/workflows/release.yml: v* tegi qo'yilganda uchala tizimda build
- [ ] Natija avtomatik GitHub Release ga yuklanadi, SHA256SUMS bilan
- [ ] Linux: ubuntu-latest. macOS: macos-latest (arm64) va Intel uchun alohida. Windows: windows-latest
- [ ] Cargo kesh (build tezligi uchun)
- [ ] Har pull requestda cargo check va testlar
- [ ] Release matni avtomatik (oxirgi commitlardan)

**Tayyor deyiladi:** v0.6.0 tegi qo'yilganda uchala fayl release'da o'zi paydo bo'ladi.

---

## 8. Bosqich 5: Funksiyalarni ko'chirish (2-4 hafta)

Tajriba muvaffaqiyatli bo'lsa, shu tartibda:

1. [ ] Umumiy yadro rail-core: tab/ustun modeli, sessiyani saqlash va tiklash, profillar, sozlamalar
2. [ ] Lenta va ustunlar, resize, vertikal stack
3. [ ] Profillar (alohida cookie), profil almashtirish
4. [ ] Omnibar/palette, tarix, qidiruv tizimlari
5. [ ] Sidebar, roadmap/minimap
6. [ ] Shortcutlar: har amalga bitta klavish, sozlamalarda o'zgartirish
7. [ ] DevTools (F12)
8. [ ] Sozlamalar, html.css muharriri, Data & Backup
9. [ ] Yangilanish kartasi (qora-oq uslub saqlanadi)

Har qadam oxirida uchala tizimda qo'lda sinov ro'yxati (pastda). Har qadam alohida commit va alohida beta versiya.

---

## 9. Bosqich 6: Tarqatish, hammasi bepul (1 hafta)

- [ ] macOS: .app ni .dmg ga o'rash, ad-hoc imzo
- [ ] Windows: oddiy .exe (keyin xohlasa installer)
- [ ] Linux: hozirgi binary + install.sh, ixtiyoriy AppImage (skript tayyor)
- [ ] Homebrew tap: brew install mrdevs2011/rail/rail
- [ ] Scoop bucket va winget manifesti
- [ ] SignPath Foundation ga ariza (Windows imzosi uchun)
- [ ] Saytdagi macOS/Windows tugmalarini yoqish

---

## 10. Qo'lda sinov ro'yxati (har release oldidan, har tizimda)

- [ ] Ochiladi, ikonka to'g'ri, oyna nomi "Rail"
- [ ] Yangi tab, URL kiritish, qidiruv
- [ ] Ikki profil: bir saytga ikki akkaunt bilan kirish, cookie aralashmaydi
- [ ] Yopib qayta ochganda tablar joyida
- [ ] Shortcutlar ishlaydi, saytga yozganda to'qnashmaydi
- [ ] F12 ochiladi
- [ ] Eski versiyadan Update, yangi versiya, tablar joyida
- [ ] rail --update ishlaydi
- [ ] Oyna kichraytirilganda va yarim ekranda UI buzilmaydi
- [ ] Sayt: yuklash tugmasi oxirgi release'ga olib boradi

---

## 11. Xavflar

| Xavf | Chorasi |
| :--- | :--- |
| wry bir oynada ko'p webview bermasa | Zaxira reja: Linuxda GTK4 qoladi, mac/win uchun alohida qobiq, umumiy rail-core |
| macOS eski versiyalarida profil izolyatsiyasi cheklangan | Minimal macOS 14 ni talab qilish |
| Mac va Windowsda o'zim sinay olmayman | CI build + do'st qo'lda sinaydi, ro'yxat yuqorida |
| Imzosiz fayl ogohlantirish chiqaradi | Saytda qadam-baqadam yo'riqnoma, SignPath ga ariza |
| Domen yo'q | Kerak emas, rail-browser.vercel.app ishlaydi |
| Vercel Hobby shartlari (tijoriy emas) | Loyiha tijoriy bo'lsa GitHub Pages ga ko'chish (docs/ papka tayyor) |
| GitHub API limiti (saytda versiya olishda) | Limit har tashrif buyuruvchi IP uchun, xato bo'lsa zaxira versiya ko'rinadi |

---

## 12. Versiya rejasi

| Versiya | Mazmuni |
| :--- | :--- |
| v0.5.x | Linux, hozirgi holat, kichik tuzatishlar |
| v0.6.0 | Sayt va tizimga qarab yangilash tayyor |
| v0.7.0 | Spike natijasi: uch tizimda demo |
| v0.9.0 | Funksiyalar ko'chirilgan, uch tizimda beta |
| v1.0.0 | Uch tizimda barqaror, Homebrew/Scoop/winget |

---

## 13. Keyingi qadamlar (tartib bilan)

1. Saytni qo'lda tekshirish (mobil va turli brauzerlar)
2. Bosqich 2: updater asset tanlash (OS/ARCH) va SHA256 tekshiruvi
3. Bosqich 3: tao + wry spike, birinchi savol: ko'p webview Waylandda
4. Spike natijasini shu faylga yozish va qarorni qat'iylashtirish (asosiy yoki zaxira reja)
5. Bosqich 4: CI workflow

## 14. Ochiq savollar

- macOS va Windowsda kim sinaydi (do'st yoki virtual mashina)?
- Spike'da Linuxdagi hozirgi silliq scroll va touchpad sifatini qanday o'lchaymiz?
