# Production.md — Rail uchun ishni yakunlash tartibi

> **AI uchun:** ishingni tugatgach SHU FAYLNI O'QI va quyidagi hammasini ketma-ket bajar.
> Foydalanuvchi faqat "Production.md ni o'qi va qil" deydi — qolganini o'zing hal qil.
> Token (`~/.gh-mrtoken.txt`) hech qachon chatda ko'rinmasin. Force push, tag/release o'chirish — faqat aniq so'ralsa.

## 0. Yagona yo'l qoidasi (DUBLIKAT YARATMA)

- Loyihaning **yagona** nusxasi: `~/Claude/work/rail-browser`. Boshqa clone/nusxa (`~/Claude/work/rail`, `/tmp/rail*`, `~/rail`) YARATMA. Vaqtinchalik narsa kerak bo'lsa ish tugagach o'chir.
- O'rnatilgan binar **faqat** `~/.local/bin/rail`. `which -a rail` da bitta yo'l chiqishi shart.
- Ilovalar menyusida **bitta** `~/.local/share/applications/rail.desktop` (eski `dev.rail.browser.desktop` bo'lmasin). Ikon bitta: `~/.local/share/icons/hicolor/scalable/apps/rail.svg`.
- O'rnatish atomik: `cp ... ~/.local/bin/rail.new && chmod +x && mv -f rail.new rail`. `.new`, `.bak`, `rail-x86_64.AppImage` nusxalari qolmasin.
- Releasega har bir tag uchun **bitta** asset: `rail-linux-x86_64` (updater aynan shu nomni qidiradi). Takror yuklama.
- Git remote'ga tegma (`https://140.82.121.4/...`, `http.sslverify=false` — bu ataylab). Tokenni `.git/config` ga yozma.

## 1. Boshlashdan oldin

1. `cd ~/Claude/work/rail-browser && ~/Claude/.bin/ai-status . claude` — qulf band bo'lsa, zo'rlab kirma.
2. `git status -sb`, `git log --oneline -3` — nima o'zgarganini bil.

## 2. Sifat tekshiruvi

1. Versiyani ko'tar (semver): `Cargo.toml` → `version`; saytdagi zaxira `docs/index.html` (`VER='...'` va `id="ver"`); `ROADMAP.md` dagi "Hozirgi holat (vX.Y.Z)".
2. `cargo build --release` — ogohlantirish/xatosiz.
3. `cargo test` — hammasi o'tishi shart (i18n jadvali to'liqligi testi ham).
4. Yangi UI matni qo'shilgan bo'lsa: `tr("English")` ishlatilgan va `src/i18n_tables.rs` ga `(en, uz, ru)` qo'shilgan (til: uz default, en, ru).

## 3. GitHub'ga push

```bash
git add -A            # .gitignore dagi target/ va .ai-lock kirmaydi
git -c user.name="MR" -c user.email="mrdevs2011@users.noreply.github.com" commit -m "feat/fix: qisqa tavsif (vX.Y.Z)"
GIT_ASKPASS="$HOME/.gh-askpass.sh" GIT_TERMINAL_PROMPT=0 git push origin HEAD:main
```

Tekshir: `git status -sb` → `## main...origin/main` (ahead/behind yo'q).

## 4. Release (GitHub Releases)

```bash
cargo build --release
packaging/release.sh "vX.Y.Z: qisqa izoh"
```

Skript: tag yaratadi (main'dan), `rail-linux-x86_64` assetini yuklaydi va GitHub `digest` ni mahalliy sha256 bilan solishtiradi. "SHA MOS KELMADI" chiqsa — to'xta, sababini top. Keyin `git fetch --tags`.

## 5. Kompyuterga o'rnatish

```bash
install -m 755 target/release/rail ~/.local/bin/rail.new && mv -f ~/.local/bin/rail.new ~/.local/bin/rail
sha256sum ~/.local/bin/rail target/release/rail     # ikkalasi bir xil bo'lsin
which -a rail                                        # faqat ~/.local/bin/rail
ls ~/.local/share/applications | grep -i rail        # faqat rail.desktop
```

Ishlayotgan Rail eski binarni xotirada ushlab turadi — foydalanuvchiga qayta ochishni ayt (o'zing majburan yopma).

## 6. Sayt (ixtiyoriy, `docs/index.html` o'zgargan bo'lsa)

Sayt: https://rail-browser.vercel.app (manba — `docs/`). Deploy imkoni bo'lmasa (Vercel kirishi yo'q) — buni hisobotda ayt, boshqa yo'l bilan aylanib o'tma.

## 7. Yakuniy tekshiruv (hammasi ✓ bo'lsin)

- [ ] `git status -sb` toza, `main...origin/main`
- [ ] GitHub `releases/latest` = yangi tag, assetda `rail-linux-x86_64` bitta, sha mos
- [ ] `~/.local/bin/rail` sha = release sha
- [ ] `which -a rail` bitta yo'l, bitta `rail.desktop`
- [ ] `~/Claude/work/` da dublikat loyiha papkasi yo'q, `/tmp` da o'zing qoldirgan fayllar yo'q
- [ ] `STATUS.md` yangilandi: `~/Claude/.bin/ai-save "nima qilindi" "keyingi qadam" . claude`

## 8. Oxirgi qadam — tree

Loyiha daraxtini chiqar (faqat qo'lda yozilgan manba, generatsiya qilingan/yuklab olingan narsalarsiz):

```bash
tree -a -I 'target|.git|node_modules|.vercel|Cargo.lock|.ai-lock|*.AppImage|rail-linux-x86_64|rail-x86_64'
```

`tree` yo'q bo'lsa: `find . -path ./target -prune -o -path ./.git -prune -o -type f -print | sort`.
