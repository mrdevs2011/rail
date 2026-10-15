# Rail Roadmap

> **Maqsad:** bitta kod, uch tizim (Linux, macOS, Windows), hammasi **bepul** yo'l bilan.
> Toza, qora-oq, dasturchilar uchun brauzer. Foydalanuvchi uchun o'rnatish va yangilash 1 bosish.

Holat belgilari: `[x]` tayyor, `[ ]` qilinadi, `[?]` tajribada tekshiriladi.

---

## Hozirgi holat (v0.5.1)

- [x] Linux: GTK4 + WebKitGTK, niri-uslubidagi 2D lenta, izolyatsiyalangan profillar
- [x] Qora-oq dev UI, ko'k faqat nuqta
- [x] Yangilanish kartasi: "N versiya orqada", **Update** bir bosishda yuklaydi va qayta ochadi
- [x] Yangilanishni tekshirish GitHub atom fidi orqali (API limiti yo'q)
- [x] Yangi ikonka: qora kvadrat, oq `r`, ko'k nuqta
- [x] `rail --update` terminaldan

Cheklov: hozirgi UI qatlami GTK4 ga bog'langan. macOS va Windowsda GTK4 + WebKitGTK ishonchli ishlamaydi, shuning uchun 3 tizim uchun qatlam almashtiriladi (3-bosqich).

---

## Bepul qoidasi

Hech narsaga pul to'lanmaydi. Ishlatiladigan bepul vositalar:

| Vazifa | Vosita | Izoh |
| :--- | :--- | :--- |
| Uch tizimda build | GitHub Actions | Ochiq repo uchun Linux, macOS va Windows runnerlari bepul |
| Sayt | GitHub Pages | `https://mrdevs2011.github.io/rail/`, keyinroq `rail.uz` ulanadi |
| Release fayllar | GitHub Releases | Hozir ham shu |
| macOS imzo | Ad-hoc (`codesign -s -`) | Pulsiz. Birinchi ochishda "Open Anyway" kerak bo'ladi |
| Windows imzo | SignPath Foundation | Ochiq loyihalarga bepul sertifikat. Ariza berilgunча imzosiz, SmartScreen "More info > Run anyway" ko'rsatadi |
| Paket menejerlar | Homebrew tap, Scoop bucket, winget | Hammasi bepul |

Pulli narsalar (Apple Developer $99/yil, Windows kod imzo sertifikati) **keyinroq, ixtiyoriy**. Ularsiz ham ishlaydi.

---

## 1-bosqich. `index.html` (rail.uz) , 1 kun

Maqsad: 5 yoshli bola ham yuklab olib ishga tushira olsin.

- [ ] `docs/index.html`, bitta fayl, kutubxonasiz, qora-oq, monospace
- [ ] Sarlavha: **Welcome to rail**
- [ ] **Select your OS** dropdown: Linux, macOS, Windows. Brauzerdan OS ni o'zi oldindan tanlaydi
- [ ] OS tanlansa: shu tizim uchun bitta katta **Download** tugmasi va 3 qadamli yo'riqnoma
  - Linux: bir qatorli `curl ... | bash`, yoki faylni yuklab `chmod +x`
  - macOS: `.dmg`/`.zip`, ilovani Applications ga tashla, birinchi ochishda o'ng tugma > Open
  - Windows: `.exe` ni och, "More info" > "Run anyway"
- [ ] Hozircha faqat Linux ishlaydi: macOS va Windows "coming soon" deb turadi
- [ ] Oxirgi versiya raqami GitHub Releases atom fididan o'zi olinadi (qo'lda yozilmaydi)
- [ ] GitHub Pages yoqiladi (`/docs` papkasidan)
- [ ] `rail.uz` ulanadi (domen egasi DNS ga `CNAME` yozadi, `docs/CNAME` fayli qo'shiladi)

**Tayyor deyiladi:** telefondan ham, kompyuterdan ham ochiladi, OS tanlash 2 bosishda yuklashga olib boradi, yo'riqnoma 3 qadamdan oshmaydi.

---

## 2-bosqich. Tizimga qarab yangilash, 1 kun

Hozir updater faqat `rail-linux-x86_64` ni biladi.

- [ ] Release asset nomlari: `rail-linux-x86_64`, `rail-macos-arm64`, `rail-macos-x86_64`, `rail-windows-x86_64.exe`
- [ ] Koddagi `std::env::consts::OS` va `ARCH` orqali o'z tizimining faylini tanlash
- [ ] Yangilanish kartasi uchala tizimda bir xil ishlaydi: **Update** > yuklaydi > qayta ochadi
- [ ] Windows: ishlab turgan `.exe` ni almashtirib bo'lmaydi, shuning uchun eskisini `rail.old.exe` ga nomini o'zgartirib, yangisini joyiga qo'yadi, keyingi ochilishda `rail.old.exe` ni o'chiradi
- [ ] macOS: yuklangan faylga `xattr -d com.apple.quarantine` va ruxsat (`chmod +x`)
- [ ] Yuklangan fayl SHA256 bilan tekshiriladi (release ichida `SHA256SUMS`)
- [ ] `rail --update` uchala tizimda

**Tayyor deyiladi:** har bir tizimda eski versiyadan yangisiga bir bosishda o'tadi, tablar joyida qoladi.

---

## 3-bosqich. Tajriba (spike), 1 hafta

Eng muhim qaror shu yerda. Shu tajriba chiqmasa, qolgan ishni boshlamaymiz.

**Reja:** brauzer yadrosi Rustda qoladi, oyna va webview uchun `tao` + `wry` ishlatiladi (Linuxda WebKitGTK, macOSda WKWebView, Windowsda WebView2).

Tekshiriladigan savollar:

- [?] Bitta oynada **bir nechta webview** yonma-yon (lenta ustunlari) uchala tizimda ishlaydimi
- [?] Linux Waylandda child webview ishlaydimi (wry GTK konteyneri orqali)
- [?] Har bir profil uchun **alohida cookie va storage**
  - Windows: har profilga alohida WebView2 `user data folder`
  - macOS: `WKWebsiteDataStore` identifikatori (macOS 14+, eskisida cheklov)
  - Linux: alohida `data directory`
- [?] Webview ustida chiziladigan UI (sidebar, palette, update kartasi) HTML sifatida
- [?] Klaviatura shortcutlari webview ichida ham ushlanadimi (sayt tugmalari bilan to'qnashmasdan)
- [?] User stylesheet (`html.css`) va devtools (F12) har uch tizimda

**Chiqish mezoni:** kichik demo: 3 ustun, 2 profil, shortcut bilan ustun almashtirish, F12. Uchala tizimda CI bilan yig'iladi va ishlaydi.

**Agar tajriba muvaffaqiyatsiz bo'lsa (zaxira reja):** Linux GTK4 versiyasi shunday qoladi, macOS va Windows uchun alohida soddaroq wry qobig'i yoziladi. Yadro (profillar, sessiya, yangilash, sozlamalar) umumiy kutubxona (`rail-core`) bo'ladi.

---

## 4-bosqich. CI: bitta kod, uchta build, 1 kun

- [ ] `.github/workflows/release.yml`: `v*` tegi qo'yilganda uchala tizimda build
- [ ] Natija avtomatik GitHub Release ga yuklanadi, `SHA256SUMS` bilan
- [ ] Linux: `ubuntu-latest`, macOS: `macos-latest` (arm64) va Intel uchun alohida, Windows: `windows-latest`
- [ ] Cargo kesh (build tezligi uchun)
- [ ] Har pull requestda `cargo check` va testlar

**Tayyor deyiladi:** `git tag v0.6.0 && git push --tags` bilan uchala fayl release'da paydo bo'ladi.

---

## 5-bosqich. Funksiyalarni ko'chirish, 2-4 hafta

Tajriba muvaffaqiyatli bo'lsa, tartib bilan:

1. [ ] Umumiy yadro: `rail-core` (tab/ustun modeli, sessiya saqlash va tiklash, profillar, sozlamalar)
2. [ ] Lenta va ustunlar, resize, vertikal stack
3. [ ] Profillar (alohida cookie), profil almashtirish
4. [ ] Omnibar/palette, tarix, qidiruv tizimlari
5. [ ] Sidebar, roadmap/minimap
6. [ ] Shortcutlar: har amalga bitta klavish, sozlamalarda o'zgartirish
7. [ ] DevTools (F12)
8. [ ] Sozlamalar, `html.css` muharriri, Data & Backup
9. [ ] Yangilanish kartasi (qora-oq uslub saqlanadi)

Har qadam oxirida: uchala tizimda qo'lda sinov ro'yxati (pastda).

---

## 6-bosqich. Tarqatish (hammasi bepul), 1 hafta

- [ ] macOS: `.app` ni `.dmg` ga o'rash, ad-hoc imzo
- [ ] Windows: oddiy `.exe` (keyin istasa installer)
- [ ] Linux: hozirgi binary + `install.sh`, ixtiyoriy AppImage
- [ ] Homebrew tap (`brew install mrdevs2011/rail/rail`)
- [ ] Scoop bucket va winget manifesti
- [ ] SignPath Foundation ga ariza (Windows imzosi uchun)

---

## Qo'lda sinov ro'yxati (har release oldidan, har tizimda)

- [ ] Ochiladi, ikonka to'g'ri, oynaning nomi "Rail"
- [ ] Yangi tab, URL kiritish, qidiruv
- [ ] Ikki profil: bir saytga ikki xil akkaunt bilan kirish, cookie aralashmaydi
- [ ] Yopib qayta ochganda tablar joyida
- [ ] Shortcutlar ishlaydi, saytga yozganda to'qnashmaydi
- [ ] F12 ochiladi
- [ ] Eski versiyadan **Update** > yangi versiya, tablar joyida
- [ ] `rail --update` ishlaydi
- [ ] Oyna kichraytirilganda va yarim ekranda UI buzilmaydi

---

## Xavflar

| Xavf | Chorasi |
| :--- | :--- |
| wry bir oynada ko'p webview bermasa | Zaxira reja: Linuxda GTK4 qoladi, mac/win uchun alohida qobiq, umumiy `rail-core` |
| macOS eski versiyalarida profil izolyatsiyasi cheklangan | Minimal macOS 14 ni talab qilish |
| Mac va Windowsda o'zim sinay olmayman | CI build + sen yoki do'st qo'lda sinaydi, sinov ro'yxati yuqorida |
| Imzosiz fayl ogohlantirish chiqaradi | Saytdagi yo'riqnomada qadam-baqadam ko'rsatiladi, SignPath ga ariza |
| Domen `rail.uz` egaligi | Shu vaqtgacha GitHub Pages manzili ishlaydi |

---

## Versiya rejasi

| Versiya | Mazmuni |
| :--- | :--- |
| v0.5.x | Linux, hozirgi holat, kichik tuzatishlar |
| v0.6.0 | Sayt (`index.html`) va tizimga qarab yangilash tayyor |
| v0.7.0 | Spike natijasi: uch tizimda demo |
| v0.9.0 | Funksiyalar ko'chirilgan, uch tizimda beta |
| v1.0.0 | Uch tizimda barqaror, Homebrew/Scoop/winget |
