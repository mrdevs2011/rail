// Rail Documentation i18n & Sync Engine
const LANGS = ['uz', 'en', 'ru'];
let currentLang = 'uz';

try {
  const urlParam = new URLSearchParams(window.location.search).get('lang');
  const stored = localStorage.getItem('rail-lang');
  if (urlParam && LANGS.includes(urlParam)) {
    currentLang = urlParam;
  } else if (stored && LANGS.includes(stored)) {
    currentLang = stored;
  }
} catch (e) {}

const syncBc = typeof BroadcastChannel !== 'undefined' ? new BroadcastChannel('rail_sync_channel') : null;

if (syncBc) {
  syncBc.onmessage = (event) => {
    if (event.data && LANGS.includes(event.data.lang) && event.data.lang !== currentLang) {
      currentLang = event.data.lang;
      applyI18n(false);
    }
  };
}

window.addEventListener('storage', (event) => {
  if (event.key === 'rail-lang' && LANGS.includes(event.newValue) && event.newValue !== currentLang) {
    currentLang = event.newValue;
    applyI18n(false);
  }
});

const DOCS_T = {
  uz: {
    // Brand & Header
    brand_sub: "hujjatlar",
    back_to_root: "← Asosiy sahifa",
    search_placeholder: "Qidirish... (/)",
    menu_open: "Menyu",

    // Sidebar
    sb_nav_guide: "Qo'llanma",
    sb_nav_reference: "Ma'lumotnoma",
    nav_intro: "Kirish & Konsepsiya",
    nav_arch: "Arxitektura & Rust",
    nav_shortcuts: "Qisqa klavishlar",
    nav_profiles: "Izolyatsiyalangan profillar",
    nav_install: "O'rnatish & CLI",
    nav_config: "Konfiguratsiya & Sozlamalar",

    // Common
    lbl_prev: "Oldingi",
    lbl_next: "Keyingi",
    lbl_copy: "Nusxa olish",
    lbl_copied: "Nusxalandi!",
    tip_title: "Maslahat",
    note_title: "Eslatma",
    warning_title: "Diqqat",

    // Index page
    idx_title: "Rail Dokumentatsiyasi",
    idx_lead: "Rail — Niri Wayland kompozitori konsepsiyasidan ilhomlangan, cheksiz gorizontal lenta, vertikal oynalar steki va 100% izolyatsiyalangan profillarga ega zamonaviy brauzer.",
    concept_h2: "01 / Nega Rail? Yangi paradigma",
    concept_p1: "An'anaviy brauzerlar (Chrome, Firefox, Safari) 20 yil oldingi tablar tizimida qotib qolgan: yuzlab tablar bir qatorda torayib, o'qib bo'lmas holatga keladi. Arc kabi brauzerlar esa vertikal panel taklif qildi, ammo u ham to'liq ekranni band qila olmaydi.",
    concept_p2: "Rail barchasini o'zgartiradi: har bir tab cheksiz gorizontal lenta (ribbon) bo'ylab joylashgan alohida to'liq ustundir. Bir ustun ichida esa oynalarni stakka (`Ctrl+V`) ajratish mumkin. Shuningdek, 100% to'liq izolyatsiyalangan profillar cookie va ma'lumotlarni hech qachon aralashtirib yubormaydi.",
    made_s: "Muhammdrasul",
    made_in_uzb: "Muhammdrasul",

    sim_h2: "Virtual 2D Lenta Simulyatori",
    sim_desc: "Rail oynalarni qanday boshqarishini quyidagi interaktiv simulyatorda sinab ko'ring. Ustun qo'shing, vertikal bo'ling va suring:",
    sim_add_col: "+ Yangi ustun (Ctrl+T)",
    sim_add_stack: "+ Vertikal stak (Ctrl+V)",
    sim_close: "Oynani yopish (Ctrl+W)",
    sim_overview: "Overview rejimi (Ctrl+O)",
    sim_hint: "Navigatsiya: [Ctrl + H/L] chap/o'ng ustun, [Ctrl + J/K] yuqori/pastki oyna",

    cards_h2: "Hujjatlar bo'limlari",
    c_arch_desc: "Rust + GTK4 + WebKitGTK 6.0 arxitekturasi, StripLayout, xotira sarfi va ichki mexanizmlar.",
    c_short_desc: "To'liq Vim-uslubidagi boshqaruv, oynalarni siljitish va qisqa klavishlar trenajyori.",
    c_prof_desc: "Har bir profil uchun alohida WebKitNetworkSession, kuki va kesh izolyatsiyasi laboratoriyasi.",
    c_inst_desc: "Linux uchun bir qatorli avtomatik o'rnatish, CLI parametrlari va hot-update yangilanishi.",
    c_conf_desc: "Konfiguratsiya fayli, temalarni sozlash, maxsus CSS va ko'p tillilik.",

    // Architecture page
    arch_title: "Arxitektura va Ichki Tuzilish",
    arch_lead: "Rail noldan GTK4, WebKitGTK 6.0 va Rust dasturlash tilida qurilgan bo'lib, uning binar hajmi atigi ~500 KB ni tashkil qiladi.",
    arch_tech_h2: "Texnologik Stek",
    arch_tech_p: "Boshqa ko'plab zamonaviy brauzerlardan (masalan Electron asosidagi) farqli ravishda, Rail xotira va protsessordan minimal foydalanadi:",
    arch_strip_h2: "2D StripLayout mexanizmi (`strip.rs`)",
    arch_strip_p: "Rail brauzerida oynalarning joylashishi maxsus GTK Layout Manager — `StripLayout` orqali boshqariladi. Bu niri Wayland kompozitorining koordinata hisoblash algoritmidan ilhomlangan:",
    arch_strip_box1: "Gorizontal lenta (Columns): Har bir ustun gorizontal bo'ylab `col_width()` formulasi asosida joylashadi. Agar ekran tor bo'lsa yoki Full rejim yoqilgan bo'lsa, to'liq eni egallanadi.",
    arch_strip_box2: "Vertikal staklar (Rows): Bitta ustun ichida bir nechta veb-oynalar vertikal stak ko'rinishida qavatma-qavat joylashadi (`Ctrl+V`).",
    arch_strip_box3: "Overview masshtablash: `Ctrl+O` bosilganda `zoom` parametri 0.65 ga tushadi va oraliq `gap(zoom)` formulasi orqali kattalashib, barcha ustunlar bitta ekranda ixcham ko'rinadi.",
    arch_profile_h2: "Profillar izolyatsiyasi (`profile.rs`)",
    arch_profile_p: "Har bir profil o'zining mustaqil `WebKitNetworkSession` instansiyasiga ega bo'lib, uning fayllari `~/.local/share/rail/profiles/{id}` papkasida saqlanadi. Kesh, kuki va WebStorage xotiralari tizim darajasida to'liq ajratilgan.",

    // Shortcuts page
    short_title: "Klaviatura va Boshqaruv (Keybindings)",
    short_lead: "Rail klaviatura orqali boshqarish uchun maxsus loyihalashtirilgan. Sichqonchasiz ishlash tezlikni 3-5 barobarga oshiradi.",
    short_tester_title: "Interaktiv Klaviatura Trenajyori",
    short_tester_p: "Quyidagi tugmalardan birini bosing yoki klaviaturada bosing — amali ko'rsatiladi:",
    short_tbl_nav: "Navigatsiya va Fokus",
    short_tbl_win: "Oynalar va Lenta Boshqaruvi",
    short_tbl_view: "Ko'rinish va Rejimlar",
    short_tbl_other: "Boshqa qulayliklar",

    // Profiles page
    prof_title: "100% Izolyatsiyalangan Multi-Profile Tizimi",
    prof_lead: "Bitta brauzer ichida bir nechta akkauntdan (shaxsiy, ishchi, mijoz) aralashmasdan va hech narsa sizib chiqmasdan foydalaning.",
    prof_lab_title: "Virtual Profil Laboratoriyasi",
    prof_lab_desc: "Quyidagi profillardan birini tanlab, sinov tizimiga kiring. Kuki va saqlangan ma'lumotlar ikkinchi profilga umuman o'tmasligini ko'rasiz:",
    prof_lab_p1: "Profil 1 (Shaxsiy)",
    prof_lab_p2: "Profil 2 (Ishchi)",
    prof_lab_login: "site.com tizimiga kirish",
    prof_lab_wipe: "Kukilarni tozalash",
    prof_lab_switched: "Tanlangan profil faollashtirildi.",
    prof_feat_1: "Nolinchi Ma'lumot Sizishi: Har bir profil alohida disk papkasi va kuki-jar ga ega.",
    prof_feat_2: "Tezkor almashtirish: URL maydonidagi rangli indikator yoki Ctrl+P orqali 1 soniyada almashtirish.",
    prof_feat_3: "Rangli identifikatorlar: Har bir profil o'zining rangli doirachasi va bosh harflari bilan belgilanadi.",

    // Installation page
    inst_title: "O'rnatish va CLI Qo'llanmasi",
    inst_lead: "Rail Linux tizimlarida o'ta yengil va birgina buyruq bilan o'rnatiladi. Windows va macOS uchun esa tez orada chiqariladi.",
    inst_m1_title: "1-usul: Bir qatorli avtomatik o'rnatish (Tavsiya etiladi)",
    inst_m1_p: "Ushbu buyruq kerakli kutubxonalarni tekshiradi, binarni `~/.local/bin/rail` ga joylaydi va tizim menyusiga ilova yorlig'ini qo'shadi:",
    inst_m2_title: "2-usul: Tayyor binar faylni yuklab olish (~500 KB)",
    inst_m2_p: "Agar tizimda `libwebkitgtk-6.0` mavjud bo'lsa, to'g'ridan-to'g'ri yuklab olib ishlatishingiz mumkin:",
    inst_cli_h2: "CLI Buyruqlari va Parametrlari",
    inst_update_h2: "Avtomatik Yangilanish Tizimi (`update.rs`)",
    inst_update_p: "Rail har ishga tushganda yangi relizni avtomatik tekshiradi. SHA256 nazorat summasini tekshirib, joriy ochiq tablarni saqlagan holda dasturni yangilaydi.",

    // Configuration page
    conf_title: "Konfiguratsiya va Sozlamalar",
    conf_lead: "Rail soddalik va moslashuvchanlik tamoyiliga tayanadi. Barcha sozlamalar JSON formatida saqlanadi.",
    conf_file_h2: "Sozlamalar Fayli Joylashuvi",
    conf_file_p: "Konfiguratsiya `~/.config/rail/config.json` manzilida saqlanadi. Uni istalgan matn muharririda ochib o'zgartirishingiz mumkin:",
    conf_i18n_h2: "Interfeys Tili (i18n)",
    conf_i18n_p: "Rail to'liq 3 ta tilni qo'llab-quvvatlaydi: O'zbekcha (sukut bo'yicha), Inglizcha va Ruscha. Tilni Settings menyusidan yoki `rail --lang <uz|en|ru>` orqali o'zgartirish mumkin.",
    conf_css_h2: "Maxsus CSS Usullar",
    conf_css_p: "Saytlarning ko'rinishini o'zingizga moslashtirish uchun foydalanuvchi CSS qoidalari `~/.config/rail/custom.css` faylida kiritilishi mumkin."
  },

  en: {
    // Brand & Header
    brand_sub: "docs",
    back_to_root: "← Home",
    search_placeholder: "Search... (/)",
    menu_open: "Menu",

    // Sidebar
    sb_nav_guide: "Guide",
    sb_nav_reference: "Reference",
    nav_intro: "Introduction & Concept",
    nav_arch: "Architecture & Rust",
    nav_shortcuts: "Keyboard Shortcuts",
    nav_profiles: "Isolated Profiles",
    nav_install: "Install & CLI",
    nav_config: "Configuration & Settings",

    // Common
    lbl_prev: "Previous",
    lbl_next: "Next",
    lbl_copy: "Copy",
    lbl_copied: "Copied!",
    tip_title: "Tip",
    note_title: "Note",
    warning_title: "Caution",

    // Index page
    idx_title: "Rail Documentation",
    idx_lead: "Rail — a blazing-fast, keyboard-driven browser inspired by the Niri Wayland compositor, featuring an infinite horizontal ribbon, vertical window stacks, and 100% isolated multi-profiles.",
    concept_h2: "01 / Why Rail? A New Paradigm",
    concept_p1: "Traditional browsers (Chrome, Firefox, Safari) have been trapped in a 20-year-old tab model: dozens of horizontal tabs shrink until titles become unreadable. Browsers like Arc introduced vertical tabs, but still confine workspaces to fixed single views.",
    concept_p2: "Rail changes everything: each tab is a full column on an infinite horizontal ribbon. Inside any column, you can vertically stack windows with `Ctrl+V`. Furthermore, 100% isolated profiles keep cookies and sessions strictly partitioned.",
    made_s: "Muhammdrasul",
    made_in_uzb: "Muhammdrasul",

    sim_h2: "Virtual 2D Ribbon Simulator",
    sim_desc: "Try how Rail manages columns and stacks in this interactive simulator. Add columns, split vertically, and slide across:",
    sim_add_col: "+ New Column (Ctrl+T)",
    sim_add_stack: "+ Vertical Stack (Ctrl+V)",
    sim_close: "Close Window (Ctrl+W)",
    sim_overview: "Overview Mode (Ctrl+O)",
    sim_hint: "Navigation: [Ctrl + H/L] left/right column, [Ctrl + J/K] up/down stack",

    cards_h2: "Documentation Sections",
    c_arch_desc: "Rust + GTK4 + WebKitGTK 6.0 architecture, StripLayout, memory footprint, and internals.",
    c_short_desc: "Full Vim-style keyboard workflow, window swapping, and interactive shortcut tester.",
    c_prof_desc: "Independent WebKitNetworkSession instances, zero cookie leakage, and interactive profile lab.",
    c_inst_desc: "One-line automated installer for Linux, CLI parameters, and zero-downtime hot updates.",
    c_conf_desc: "Configuration files, UI themes, custom CSS stylesheets, and localization.",

    // Architecture page
    arch_title: "Architecture & Rust Internals",
    arch_lead: "Rail is built from scratch in GTK4, WebKitGTK 6.0, and Rust, producing a lean native binary of just ~500 KB.",
    arch_tech_h2: "Technology Stack",
    arch_tech_p: "Unlike bloated Electron or Chromium-wrapper browsers, Rail uses minimal RAM and CPU while delivering buttery 60+ FPS native rendering:",
    arch_strip_h2: "2D StripLayout Mechanism (`strip.rs`)",
    arch_strip_p: "The window placement in Rail is powered by a custom GTK Layout Manager named `StripLayout`, inspired by Niri Wayland compositor geometry:",
    arch_strip_box1: "Horizontal Ribbon (Columns): Columns slide smoothly along the X axis calculated via `col_width()`. If screen is narrow or full mode is toggled, it dynamically expands to full viewport width.",
    arch_strip_box2: "Vertical Stacks (Rows): Inside any column, multiple web views can be vertically partitioned into stacks (`Ctrl+V`).",
    arch_strip_box3: "Overview Zoom: When `Ctrl+O` is pressed, zoom drops to 0.65 and gap expands via `gap(zoom)` so all workspaces fit into a bird's-eye view.",
    arch_profile_h2: "Profile Isolation (`profile.rs`)",
    arch_profile_p: "Each profile has its own dedicated `WebKitNetworkSession` with data saved in `~/.local/share/rail/profiles/{id}`. Cache, cookies, and WebStorage are strictly isolated at the OS and WebKit engine level.",

    // Shortcuts page
    short_title: "Keyboard Shortcuts & Workflow",
    short_lead: "Rail is purposefully engineered for keyboard-first navigation. Browsing without touching the mouse multiplies your velocity.",
    short_tester_title: "Interactive Shortcut Tester",
    short_tester_p: "Click any key combo or press it on your keyboard to trigger the action:",
    short_tbl_nav: "Navigation & Focus",
    short_tbl_win: "Window & Ribbon Management",
    short_tbl_view: "Views & Modes",
    short_tbl_other: "Utilities",

    // Profiles page
    prof_title: "100% Isolated Multi-Profile System",
    prof_lead: "Run multiple accounts (work, personal, clients) inside one unified browser window without cross-contamination.",
    prof_lab_title: "Virtual Profile Isolation Lab",
    prof_lab_desc: "Pick a profile below and sign in. Notice how cookies and stored data remain completely invisible to other profiles:",
    prof_lab_p1: "Profile 1 (Personal)",
    prof_lab_p2: "Profile 2 (Work)",
    prof_lab_login: "Sign in to site.com",
    prof_lab_wipe: "Clear cookies",
    prof_lab_switched: "Active profile switched.",
    prof_feat_1: "Zero Cookie Leaks: Each profile holds its own cookie jar, cache directory, and local storage pool.",
    prof_feat_2: "Instant Switcher: Switch in less than a second via Ctrl+P or the color badge in the omnibar.",
    prof_feat_3: "Visual Accents: Every profile is marked with distinct accent colors and monogram avatars.",

    // Installation page
    inst_title: "Installation & CLI Guide",
    inst_lead: "Rail installs on Linux in seconds via a one-line installer. Windows and macOS support is on the active roadmap.",
    inst_m1_title: "Method 1: One-line Automatic Installer (Recommended)",
    inst_m1_p: "This script verifies prerequisites, copies the binary to `~/.local/bin/rail`, and adds a desktop menu entry:",
    inst_m2_title: "Method 2: Standalone Binary (~500 KB)",
    inst_m2_p: "If your system already has `libwebkitgtk-6.0`, simply download the standalone release binary:",
    inst_cli_h2: "CLI Flags and Parameters",
    inst_update_h2: "Automatic Update Engine (`update.rs`)",
    inst_update_p: "On launch, Rail checks GitHub Releases. It verifies SHA256 checksums and performs seamless in-place updates without losing open tabs.",

    // Configuration page
    conf_title: "Configuration & Customization",
    conf_lead: "Rail adheres to simplicity and flexibility. All configuration parameters are persisted in human-readable JSON.",
    conf_file_h2: "Config File Location",
    conf_file_p: "Configuration is stored at `~/.config/rail/config.json`. Edit it with any text editor:",
    conf_i18n_h2: "Interface Language (i18n)",
    conf_i18n_p: "Rail natively supports 3 languages: Uzbek (default), English, and Russian. Toggle anytime in Settings or via `rail --lang <uz|en|ru>`.",
    conf_css_h2: "Custom CSS Overrides",
    conf_css_p: "Inject custom user styles into web pages by editing `~/.config/rail/custom.css`."
  },

  ru: {
    // Brand & Header
    brand_sub: "документация",
    back_to_root: "← Главная",
    search_placeholder: "Поиск... (/)",
    menu_open: "Меню",

    // Sidebar
    sb_nav_guide: "Руководство",
    sb_nav_reference: "Справочник",
    nav_intro: "Введение и концепция",
    nav_arch: "Архитектура и Rust",
    nav_shortcuts: "Горячие клавиши",
    nav_profiles: "Изолированные профили",
    nav_install: "Установка и CLI",
    nav_config: "Конфигурация и настройки",

    // Common
    lbl_prev: "Назад",
    lbl_next: "Далее",
    lbl_copy: "Копировать",
    lbl_copied: "Скопировано!",
    tip_title: "Совет",
    note_title: "Заметка",
    warning_title: "Внимание",

    // Index page
    idx_title: "Документация Rail",
    idx_lead: "Rail — сверхбыстрый браузер с управлением с клавиатуры, вдохновленный оконным композитором Niri Wayland, с бесконечной горизонтальной лентой, вертикальными стопками и 100% изолированными профилями.",
    concept_h2: "01 / Почему Rail? Новая парадигма",
    concept_p1: "Традиционные браузеры (Chrome, Firefox, Safari) застряли в парадигме 20-летней давности: десятки вкладок сжимаются до неузнаваемости. Браузеры вроде Arc предложили вертикальные вкладки, но по-прежнему сковывают экран жесткими рамками.",
    concept_p2: "Rail меняет всё: каждая вкладка — это полноценная колонка на бесконечной горизонтальной ленте. Внутри колонки окна можно делить вертикально на стопки с помощью `Ctrl+V`. А 100% изолированные профили гарантируют отсутствие утечек данных.",
    made_s: "Muhammdrasul",
    made_in_uzb: "Muhammdrasul",

    sim_h2: "Виртуальный симулятор 2D ленты",
    sim_desc: "Попробуйте управление колонками и стопками в интерактивном симуляторе. Добавляйте колонки, делите вертикально и перемещайтесь:",
    sim_add_col: "+ Новая колонка (Ctrl+T)",
    sim_add_stack: "+ Стопка окон (Ctrl+V)",
    sim_close: "Закрыть окно (Ctrl+W)",
    sim_overview: "Режим обзора (Ctrl+O)",
    sim_hint: "Навигация: [Ctrl + H/L] левая/правая колонка, [Ctrl + J/K] верхнее/нижнее окно",

    cards_h2: "Разделы документации",
    c_arch_desc: "Архитектура Rust + GTK4 + WebKitGTK 6.0, StripLayout, расход памяти и внутреннее устройство.",
    c_short_desc: "Полное управление в стиле Vim, перемещение окон и тренажер горячих клавиш.",
    c_prof_desc: "Независимые сессии WebKitNetworkSession, нулевая утечка cookie и интерактивная лаборатория.",
    c_inst_desc: "Установка одной командой для Linux, параметры CLI и бесшовное горячее обновление.",
    c_conf_desc: "Файлы конфигурации, темы оформления, пользовательский CSS и локализация.",

    // Architecture page
    arch_title: "Архитектура и ядро Rust",
    arch_lead: "Rail построен с нуля на GTK4, WebKitGTK 6.0 и языке Rust. Размер бинарного файла составляет всего ~500 КБ.",
    arch_tech_h2: "Технологический стек",
    arch_tech_p: "В отличие от тяжелых браузеров на Electron или Chromium, Rail потребляет минимум ресурсов и выдает плавные нативные 60+ FPS:",
    arch_strip_h2: "Механизм 2D StripLayout (`strip.rs`)",
    arch_strip_p: "Размещение окон управляется специальным макетом GTK — `StripLayout`, геометрия которого вдохновлена композитором Niri Wayland:",
    arch_strip_box1: "Горизонтальная лента (Columns): Колонки плавно скользят по оси X на основе расчета `col_width()`. В узком окне или полноэкранном режиме колонка занимает всю ширину.",
    arch_strip_box2: "Вертикальные стопки (Rows): Внутри колонки несколько веб-страниц могут делиться на вертикальные стопки (`Ctrl+V`).",
    arch_strip_box3: "Масштаб обзора: При нажатии `Ctrl+O` зум уменьшается до 0.65, а зазоры `gap(zoom)` расширяются для удобного обзора всех рабочих пространств.",
    arch_profile_h2: "Изоляция профилей (`profile.rs`)",
    arch_profile_p: "Каждый профиль использует независимый экземпляр `WebKitNetworkSession` с данными в `~/.local/share/rail/profiles/{id}`. Кэш, cookie и WebStorage строго разделены.",

    // Shortcuts page
    short_title: "Горячие клавиши и управление",
    short_lead: "Rail спроектирован для максимальной скорости работы без мыши. Навигация в стиле Vim ускоряет сёрфинг в 3-5 раз.",
    short_tester_title: "Интерактивный тренажер клавиш",
    short_tester_p: "Нажмите любую комбинацию или клавиши на клавиатуре для проверки действия:",
    short_tbl_nav: "Навигация и фокус",
    short_tbl_win: "Управление окнами и лентой",
    short_tbl_view: "Режимы и отображение",
    short_tbl_other: "Служебные команды",

    // Profiles page
    prof_title: "100% Изолированные профили",
    prof_lead: "Работайте с несколькими аккаунтами (личный, рабочий, клиентский) в одном окне без смешивания данных.",
    prof_lab_title: "Виртуальная лаборатория профилей",
    prof_lab_desc: "Выберите профиль ниже и войдите на сайт. Cookie и локальное хранилище никогда не попадут в соседний профиль:",
    prof_lab_p1: "Профиль 1 (Личный)",
    prof_lab_p2: "Профиль 2 (Рабочий)",
    prof_lab_login: "Войти на site.com",
    prof_lab_wipe: "Очистить cookie",
    prof_lab_switched: "Активный профиль переключен.",
    prof_feat_1: "Нулевая утечка: У каждого профиля собственная база cookie, директория кэша и хранилище.",
    prof_feat_2: "Мгновенное переключение: Переключение за 1 секунду по Ctrl+P или кнопке в омнибаре.",
    prof_feat_3: "Цветовые метки: Каждый профиль выделен уникальным цветом и монограммой.",

    // Installation page
    inst_title: "Установка и справочник CLI",
    inst_lead: "Rail устанавливается в Linux за пару секунд одной командой. Поддержка Windows и macOS находится в активной разработке.",
    inst_m1_title: "Способ 1: Автоматическая установка одной строкой (Рекомендуется)",
    inst_m1_p: "Скрипт проверяет зависимости, копирует бинарник в `~/.local/bin/rail` и добавляет ярлык в меню приложений:",
    inst_m2_title: "Способ 2: Готовый бинарный файл (~500 КБ)",
    inst_m2_p: "Если в вашей системе установлен `libwebkitgtk-6.0`, можно сразу скачать и запустить бинарник:",
    inst_cli_h2: "Параметры и флаги CLI",
    inst_update_h2: "Система автообновления (`update.rs`)",
    inst_update_p: "При каждом запуске Rail проверяет релизы на GitHub, валидирует контрольную сумму SHA256 и обновляется на лету без потери открытых вкладок.",

    // Configuration page
    conf_title: "Конфигурация и кастомизация",
    conf_lead: "Rail ценит простоту и прозрачность. Все пользовательские настройки хранятся в формате JSON.",
    conf_file_h2: "Расположение конфигурационного файла",
    conf_file_p: "Настройки сохраняются в `~/.config/rail/config.json`. Их можно редактировать в любом текстовом редакторе:",
    conf_i18n_h2: "Язык интерфейса (i18n)",
    conf_i18n_p: "Rail поддерживает 3 языка: узбекский (по умолчанию), английский и русский. Язык можно сменить в настройках или командой `rail --lang <uz|en|ru>`.",
    conf_css_h2: "Пользовательские стили CSS",
    conf_css_p: "Настройте отображение любых веб-страниц, добавив свои CSS правила в `~/.config/rail/custom.css`."
  }
};

function docT(key) {
  const dict = DOCS_T[currentLang] || DOCS_T.uz;
  return dict[key] != null ? dict[key] : (DOCS_T.uz[key] || key);
}

function applyI18n(broadcast = true) {
  document.documentElement.lang = currentLang;

  // Update text nodes
  document.querySelectorAll('[data-t]').forEach(el => {
    const key = el.getAttribute('data-t');
    el.textContent = docT(key);
  });

  // Update HTML nodes
  document.querySelectorAll('[data-th]').forEach(el => {
    const key = el.getAttribute('data-th');
    el.innerHTML = docT(key);
  });

  // Update placeholders
  document.querySelectorAll('[data-ph]').forEach(el => {
    const key = el.getAttribute('data-ph');
    el.setAttribute('placeholder', docT(key));
  });

  // Update language buttons active state
  document.querySelectorAll('.lang button').forEach(btn => {
    btn.classList.toggle('on', btn.getAttribute('data-l') === currentLang);
  });

  // Notify custom listeners if any
  window.dispatchEvent(new CustomEvent('docs-lang-changed', { detail: { lang: currentLang } }));

  if (broadcast) {
    try {
      localStorage.setItem('rail-lang', currentLang);
    } catch (e) {}
    if (syncBc) {
      syncBc.postMessage({ lang: currentLang });
    }
  }
}

function setDocsLang(lang) {
  if (!LANGS.includes(lang)) return;
  currentLang = lang;
  applyI18n(true);
}

function bootDocsI18n() {
  document.querySelectorAll('.lang button').forEach(btn => {
    if (btn.dataset.i18nBound === '1') return;
    btn.dataset.i18nBound = '1';
    btn.type = 'button';
    btn.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      setDocsLang(btn.getAttribute('data-l'));
    });
  });
  applyI18n(false);
}
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', bootDocsI18n);
} else {
  bootDocsI18n();
}
