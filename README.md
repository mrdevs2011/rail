# Rail

> **Rail** — zamonaviy Linux tizimlari uchun mo'ljallangan, yuqori tezlikka ega, xotirani tejaydigan va to'liq klaviaturaga yo'naltirilgan minimalist brauzer.

O'zbekistonlik dasturchi **Muhammadrasul Qosimov** tomonidan sof Rust, GTK4 va WebKitGTK 6.0 negizida yaratilgan. Loyiha tashqi pullik servislardan 100% mustaqil bo'lib, Wayland (Niri) va X11 muhitlarida ortiqcha yuklamalarsiz (0% idle CPU) ishlaydi.

---

## ⚡ Asosiy Muhandislik Ustunlari

- **Ko'p Profilli To'liq Izolyatsiya (Sandboxed Profiles)**  
  Har bir profil uchun alohida `WebKitNetworkSession` ajratiladi. Ish, shaxsiy yoki o'qish profillari kuki, kesh va `localStorage` bo'yicha mutlaq ajratilgan bo'lib, ma'lumotlar oqishi (leak) ning oldi olinadi.

- **Niri Uslubidagi 2D Tiling va Klaviaturaga Yo'naltirilgan Boshqaruv**  
  Gorizontal cheksiz lenta (horizontal ribbon) va vertikal oynalar steki. Sichqonchasiz tezkor boshqaruv, buyruqlar palitrasi (`Ctrl+T` / `Ctrl+L`) va sozlanuvchi tugmalar kombinatsiyalari.

- **Atomik Tranzaksiyaviy Saqlash va Auto-Healing**  
  Sessiyalar, sozlamalar va profillar xavfsiz atomik tranzaksiyalar (yozish, tasdiqlash va zaxira tiklash) orqali saqlanadi. Kutilmagan uzilish yoki nosozlik yuz berganda brauzer o'z holatini avtomatik ravishda tiklaydi.

- **Resurslarni Minimal Sarflash va Sof Native Arxitektura**  
  Electron yoki keraksiz qatlamlarsiz, binar fayl hajmi juda ixcham, xotira sarfi minimal va fonga o'tgan tablar avtomatik ravishda uyqu rejimiga o'tkaziladi.

---

## 🚀 Tizim Talablari va O'rnatish

### 1. Kerakli Kutubxonalar (Dependencies)

Rail'ni yig'ish va ishlatish uchun tizimda GTK4 va WebKitGTK 6.0 bo'lishi lozim:

- **Arch Linux / Manjaro:**
  ```bash
  sudo pacman -S gtk4 webkitgtk-6.0 base-devel
  ```

- **Ubuntu 23.04+ / Debian 12+:**
  ```bash
  sudo apt update
  sudo apt install libgtk-4-dev libwebkitgtk-6.0-dev build-essential
  ```

- **Fedora 38+:**
  ```bash
  sudo dnf install gtk4-devel webkitgtk6.0-devel gcc
  ```

### 2. Manba Kodidan Yig'ish (Build from Source)

```bash
# 1. Omborxona nusxasini oling
git clone https://github.com/mrdevs2011/rail.git
cd rail

# 2. Release rejimida yig'ing
cargo build --release

# 3. Brauzerni ishga tushiring
cargo run --release
```

Tizimga o'rnatish uchun yig'ilgan binar faylni yo'lga ko'chiring:
```bash
install -Dm755 target/release/rail ~/.local/bin/rail
```

---

## ⌨️ Asosiy Tugmalar (Keybindings)

| Kombinatsiya | Amal |
| :--- | :--- |
| `Ctrl + T` / `Ctrl + L` | Omnibar / Manzil satri (qidiruv va tezkor buyruqlar) |
| `Ctrl + W` | Fokusdagi tabni yopish |
| `Ctrl + R` / `Ctrl + Shift + R` | Sahifani yangilash / Keshsiz qat'iy yangilash |
| `Alt + H` / `Alt + L` | Chap / o'ng ustunga o'tish |
| `Alt + J` / `Alt + K` | Ustun ichidagi stekda pastki / yuqori oynaga o'tish |
| `Alt + Shift + V` | Yangi ish maydoni (workspace) yaratish |
| `Alt + O` | Fazoviy Overview rejimi (umumiy ko'rinish) |
| `Alt + P` | Tezkor profillar popover menyusi |
| `Alt + S` | Tezkor sozlamalar popover menyusi |
| `Alt + B` | Chap panelni (Sidebar) yashirish yoki ko'rsatish |
| `F12` | WebKit dasturchi asboblari (DevTools) |

---

## 🌐 Til va Mahalliylashtirish (i18n)

Rail to'liq 3 ta tilda ishlaydi:
- **O'zbekcha** (standart)
- **English**
- **Русский**

Har bir til interfeys bo'ylab 100% qamrab olingan bo'lib, boshqa til aralashuvi mavjud emas. Tilni brauzer ichidagi sozlamalardan (`Alt+S` → *General*) osongina o'zgartirish mumkin.

---

## 📦 Zaxira va Ko'chirish (Backup & Sync)

Rail barcha ish maydonlari, tablar, hisoblar va kuki fayllarini bitta siqilgan arxivga eksport yoki import qilish imkoniyatini beradi:

```bash
# Barcha ma'lumotlarni eksport qilish
rail --export ~/rail_backup.tar.gz

# Ma'lumotlarni qayta tiklash
rail --import ~/rail_backup.tar.gz

# Saqlash joyi va holat haqida ma'lumot
rail --data-info
```

---

## 🌍 English Overview & Quickstart

**Rail** is a modern, keyboard-centric, high-performance Linux web browser engineered in pure Rust with GTK4 and WebKitGTK 6.0 by Uzbek engineer Muhammadrasul Qosimov. It features containerized multi-profile storage isolation, Niri-inspired 2D infinite tiling, zero idle CPU consumption, and resilient self-healing session persistence.

### Quick Build:
```bash
git clone https://github.com/mrdevs2011/rail.git
cd rail
cargo build --release
./target/release/rail
```

---

## 📄 Litsenziya

Loyiha [MIT Litsenziyasi](LICENSE) asosida ochiq manbali tarzda tarqatiladi.
