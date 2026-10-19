# Rail

> **Rail** — a blazing-fast, keyboard-driven web browser built with GTK4 and WebKitGTK. Inspired by the **Niri** Wayland compositor, it features an infinite horizontal ribbon of columns, vertical window stacks, and storage-isolated multi-profiles.

---

## 🚀 Quick Install (Tezkor o‘rnatish)

### Usul 1: Bir qatorli avtomatik o‘rnatish (Tavsiya etiladi)

Terminalni oching va quyidagi buyruqni bering. Skript kerakli kutubxonalarni tekshiradi, dasturni `~/.local/bin/rail` ga joylaydi va ilovalar menyusiga (`.desktop` fayl va ikonka) qo‘shadi:

```bash
curl -fsSL https://raw.githubusercontent.com/mrdevs2011/rail/main/install.sh | bash
```

### Usul 2: Binar faylni to‘g‘ridan-to‘g‘ri yuklab olish (~500 KB)

Agar tizimingizda `libwebkitgtk-6.0` mavjud bo‘lsa, to‘g‘ridan-to‘g‘ri reliz binar faylini yuklab olishingiz mumkin:

```bash
mkdir -p ~/.local/bin
curl -fL -o ~/.local/bin/rail https://github.com/mrdevs2011/rail/releases/latest/download/rail-linux-x86_64
chmod +x ~/.local/bin/rail
```

---

## 🔄 Yangilanish (Update)

Har safar Rail ochilganda yangi versiya tekshiriladi. Bor bo'lsa pastki o'ngda karta chiqadi: "v0.5.1 is out, you are on v0.5.0, 3 versions behind".

- **Update**: yangi versiyani yuklaydi va Rail o'zi qayta ochiladi (tablar joyida).
- **x**: kartani yopadi, keyingi ochilishda yana eslatadi.
- Terminaldan: `rail --update`

---

## 🏃 Ishga tushirish (Usage)

Terminaldan:
```bash
# Yangi oynada bosh sahifa bilan ochish
rail

# Ma'lum bir manzilni ochish
rail https://github.com
```

Yoki ilovalar ro‘yxatidan (Rofi / Wofi / GNOME / KDE) **Rail** deb qidirib ochishingiz mumkin.

---

## ⚡ Asosiy imkoniyatlar (Features)

- **Niri-Style 2D Tiling**:
  - Gorizontal cheksiz lenta (horizontal strip): har bir tab alohida ustun ko‘rinishida.
  - Har bir ustun ichida vertikal oynalar steki (`Ctrl+V` yoki `Ctrl+Shift+T`).
  - To‘liq klaviatura orqali boshqaruv (`Ctrl+H/J/K/L`) va oynalarni siljitish (`Ctrl+Shift+H/J/K/L`).
- **Saqlash izolyatsiyasiga ega Multi-Profile tizimi**:
  - Har bir profil uchun to‘liq alohida kuki, kesh, localStorage va sessiyalar (`WebKitNetworkSession`).
  - Profillar menejeri: `Ctrl+P` yoki `Ctrl+Shift+P`.
  - URL kiritish maydonida dumaloq profil tugmasi orqali tezkor almashtirish.
- **Responsiv va moslashuvchan dizayn**:
  - Kichik ekranlar, tor oynalar va yarim ekran (half-screen snapping) holatlarida ham qulay ishlaydi.
- **Yengil va tezkor**:
  - Rust + GTK4 + WebKitGTK 6.0 negizida yaratilgan.
  - Binar fayl hajmi atigi ~500 KB.

---

## ⌨️ Klaviatura qisqa klavishlari (Keybindings)

| Klavishlar | Vazifasi |
| :--- | :--- |
| `Ctrl + T` | Omnibar / yangi tab |
| `Ctrl + L` | Manzil qatori (URL bar) |
| `Ctrl + W` | Fokusdagi tabni yopish |
| `Ctrl + R` | Sahifani yangilash (Reload) |
| `Ctrl + +` / `Ctrl + -` / `Ctrl + 0` | Sahifa masshtabi 20%–500% (overview'da — overview masshtabi), 0 — asl holat |
| `Alt + F` | To‘liq kenglik / standart kenglik |
| `Alt + Shift + V` | Yangi ish maydoni (workspace) |
| `Alt + H` / `Alt + L` | Chap / o‘ng ustunga fokus |
| `Alt + J` / `Alt + K` | Ustun ichida pastdagi / yuqoridagi oynaga fokus (ustun chetida — pastki / yuqori workspace) |
| `Alt + [` / `Alt + ]` | **Stack:** oynani chap / o‘ng ustunga yig‘ish, stackdagi oynani chapga / o‘ngga ajratish (niri'dagi `consume-or-expel-window`) |
| `Alt + Shift + H` / `L` | Butun ustunni chapga / o‘ngga ko‘chirish |
| `Alt + Shift + J` / `K` | Oynani stack ichida pastga / yuqoriga surish (stack bo‘lmasa — pastki / yuqori workspace'ga) |
| `Ctrl + Tab` / `Ctrl + Shift + Tab` | Keyingi / oldingi tab (aylanma) |
| `Ctrl + 1…9` | Joriy qatordagi N-tabga o‘tish (`Ctrl + 9` — oxirgi) |
| `Alt + 1…9` / `Alt + Shift + 1…9` | N-ish maydoniga o‘tish / tabni shu yerga ko‘chirish |
| `Alt + O` | Fazoviy Overview (zoom-out) |
| `Alt + B` | Yon panelni (Sidebar) yashirish/ko‘rsatish |
| `Alt + M` | Roadmap / Minimap |
| `Alt + P` / `Alt + I` / `Alt + S` | Profillar / Data & Backup / Sozlamalar |
| `Alt + Shift + T` | Touchpad kanvas navigatsiyasi |
| `F12` / `Alt + Shift + D` | DevTools / DevTools joylashuvini almashtirish |
| `F11` | To‘liq ekran |
| `Alt + Left` / `Alt + Right` | Orqaga / Oldinga |
| `Ctrl + Q` | Rail'dan chiqish |

Overview ichida: `h` `j` `k` `l` — harakat, `Enter` — ochish, `x` — yopish, `Esc` — bekor qilish.
Har bir amalga **bitta** shortcut biriktirilgan; xohlasangiz Sozlamalar → Customize → Shortcuts orqali o‘zgartirishingiz mumkin.
---

## 🛠 Manba kodidan yig‘ish (Building from Source)

Talablar:
- Rust (Cargo 1.85+)
- GTK4 development paketlari (`gtk4-devel` yoki `libgtk-4-dev`)
- WebKitGTK 6.0 development paketlari (`webkitgtk-6.0-devel` yoki `libwebkitgtk-6.0-dev`)

```bash
git clone https://github.com/mrdevs2011/rail.git
cd rail
cargo build --release
```

Yig‘ilgan binar fayl: `target/release/rail`

Qo‘llanma: batafsil holat uchun `STATUS.md` va `ROADMAP.md` fayllariga qarang.
