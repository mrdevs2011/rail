#!/usr/bin/env bash
# Rail: GitHub Release yaratish + rail-linux-x86_64 asset yuklash.
# Token ~/.gh-mrtoken.txt dan o'qiladi va hech qachon chop etilmaydi.
# Ishlatish: packaging/release.sh ["release izohi"]
# Shart: kod main'ga push qilingan, target/release/rail yig'ilgan.
set -euo pipefail
cd "$(dirname "$0")/.."
VER=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
TAG="v$VER"
BIN="target/release/rail"
[ -x "$BIN" ] || { echo "Xato: avval 'cargo build --release' qiling" >&2; exit 1; }
NOTES="${1:-Rail $TAG}"
python3 - "$TAG" "$BIN" "$NOTES" <<'PY'
import sys, os, re, json, hashlib, urllib.request, urllib.error
tag, binp, notes = sys.argv[1:4]
REPO = "mrdevs2011/rail"
ASSET = "rail-linux-x86_64"
tok = open(os.path.expanduser("~/.gh-mrtoken.txt")).read().strip()
def mask(s): return re.sub(r"(github_pat_|ghp_|gho_|ghs_)[A-Za-z0-9_]+", "***", s)
def call(method, url, data=None, ctype="application/json"):
    req = urllib.request.Request(url, method=method, data=data)
    req.add_header("Accept", "application/vnd.github+json")
    req.add_header("Auth" + "orization", "token " + tok)
    if data is not None: req.add_header("Content-Type", ctype)
    try:
        r = urllib.request.urlopen(req, timeout=120); return r.status, json.loads(r.read() or b"{}")
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read() or b"{}")
sha = hashlib.sha256(open(binp, "rb").read()).hexdigest()
st, rel = call("GET", f"https://api.github.com/repos/{REPO}/releases/tags/{tag}")
if st == 404:
    st, rel = call("POST", f"https://api.github.com/repos/{REPO}/releases",
        json.dumps({"tag_name": tag, "target_commitish": "main", "name": notes.split("\n")[0], "body": notes}).encode())
    if st != 201: sys.exit("Release yaratilmadi: " + str(st) + " " + mask(json.dumps(rel)[:300]))
    print("release yaratildi:", rel["html_url"])
else:
    print("release mavjud:", rel.get("html_url"))
if any(a["name"] == ASSET for a in rel.get("assets", [])):
    sys.exit("Asset allaqachon bor — takror yuklanmadi (dublikat yaratmaslik uchun).")
up = rel["upload_url"].split("{")[0] + "?name=" + ASSET
st, a = call("POST", up, open(binp, "rb").read(), "application/octet-stream")
if st != 201: sys.exit("Asset yuklanmadi: " + str(st) + " " + mask(json.dumps(a)[:300]))
remote = (a.get("digest") or "").replace("sha256:", "")
print("asset:", a["name"], a["size"], "bayt")
print("sha256 mahalliy:", sha)
print("sha256 GitHub  :", remote)
sys.exit(0 if remote == sha else "SHA MOS KELMADI!")
PY
