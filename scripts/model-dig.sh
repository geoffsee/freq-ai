#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

CLIS=(claude cline codex copilot gemini grok junie xai)

have() {
  command -v "$1" >/dev/null 2>&1
}

realpath_fallback() {
  local p="$1"
  if command -v realpath >/dev/null 2>&1; then
    realpath "$p"
  elif command -v python3 >/dev/null 2>&1; then
    python3 - "$p" <<'PY'
import os, sys
print(os.path.realpath(sys.argv[1]))
PY
  else
    echo "$p"
  fi
}

extract_models() {
  tr '[:upper:]' '[:lower:]' |
  perl -ne '
    while (/\b(
      claude-(?:instant-\d+(?:\.\d+)?(?:-\d+k)?|
               [234](?:-[357])?-(?:haiku|sonnet|opus)(?:-\d{8})?(?:-v\d+)?|
               haiku(?:-\d(?:-\d)?)?(?:-\d{8})?(?:-v\d+)?|
               sonnet-(?:3-7|4(?:-\d)?)(?:-\d{8})?(?:-v\d+)?|
               opus-(?:4(?:-\d)?)(?:-\d{8})?(?:-v\d+)?|
               (?:haiku|sonnet|opus)-\d(?:-\d)?(?:-\d{8})?(?:-v\d+)?|
               (?:haiku|sonnet|opus))
      | gemini-(?:\d+(?:\.\d+)?-(?:flash|pro)(?:-[a-z0-9]+)*(?:-\d{2}-\d{2}|\-\d{3}|\-\d{4})?)
      | gpt-(?:\d+(?:\.\d+)?(?:-[a-z0-9]+)*(?:-\d{4}-\d{2}-\d{2})?)
      | grok-(?:\d+(?:-\d+)?(?:-[a-z0-9]+)*)
      | sonnet|opus|haiku|gpt|grok
    )\b/xg) {
      print "$1\n";
    }
  ' |
  sed -E '
    s/\.$//;
    s/-$//;
  ' |
  grep -Ev '(cannot|sqlite|example|failed|account|desktop|review|settings|context|user|voice|staging|folder|hiring|actions|hidden|http|local|native|proactive|prompt|socks|swift|allowed)$' |
  grep -Ev '(e\.g|eg)$' |
  awk 'length($0) > 2' |
  sort -u
}

scan_one_file() {
  local file="$1"
  [[ -f "$file" ]] || return 0
  strings "$file" 2>/dev/null | extract_models || true
}

scan_cli_to_file() {
  local cli="$1"
  local out="$2"

  : > "$out"

  if ! have "$cli"; then
    return 0
  fi

  local exe resolved root
  exe="$(command -v "$cli")"
  resolved="$(realpath_fallback "$exe")"
  root="$(dirname "$resolved")"

  scan_one_file "$resolved" >> "$out"

  find "$root" -maxdepth 4 \
    \( -name '*.js' -o -name '*.cjs' -o -name '*.mjs' -o -name '*.json' \) \
    -type f 2>/dev/null |
    while IFS= read -r f; do
      scan_one_file "$f"
    done >> "$out"

  sort -u -o "$out" "$out"
}

json_escape() {
  python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))'
}

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

# ── Raw scan (stdout) ─────────────────────────────────────────────
printf '{\n'

first_cli=1
for cli in "${CLIS[@]}"; do
  outfile="$tmpdir/$cli.txt"
  scan_cli_to_file "$cli" "$outfile"

  if [[ $first_cli -eq 0 ]]; then
    printf ',\n'
  fi
  first_cli=0

  printf '  "%s": {\n' "$cli"

  if have "$cli"; then
    exe="$(command -v "$cli")"
    resolved="$(realpath_fallback "$exe")"
    printf '    "installed": true,\n'
    printf '    "executable": %s,\n' "$(printf '%s' "$exe" | json_escape)"
    printf '    "resolved": %s,\n' "$(printf '%s' "$resolved" | json_escape)"
  else
    printf '    "installed": false,\n'
    printf '    "executable": null,\n'
    printf '    "resolved": null,\n'
  fi

  printf '    "models": ['

  if [[ -s "$outfile" ]]; then
    first_model=1
    while IFS= read -r model; do
      [[ -z "$model" ]] && continue
      if [[ $first_model -eq 0 ]]; then
        printf ', '
      fi
      first_model=0
      printf '%s' "$(printf '%s' "$model" | json_escape)"
    done < "$outfile"
  fi

  printf ']\n'
  printf '  }'
done

printf '\n}\n'

# ── Curated JSON → assets/available-models.json ──────────────────
python3 - "$tmpdir" "$REPO_ROOT/assets/available-models.json" << 'CURATE'
import json, sys, os, re

tmpdir = sys.argv[1]
out_path = sys.argv[2]

def read_models(cli):
    path = os.path.join(tmpdir, f"{cli}.txt")
    if not os.path.isfile(path):
        return set()
    with open(path) as f:
        return {line.strip() for line in f if line.strip()}

# ── Labelling ─────────────────────────────────────────────────────

def label_claude(m):
    r = m.removeprefix("claude-")
    r = re.sub(r"-\d{8}$", "", r)
    parts = r.split("-")
    tier = parts[0].title()
    ver = ".".join(parts[1:]) if len(parts) > 1 else ""
    return f"{tier} {ver}".strip()

def label_gpt(m):
    r = m.removeprefix("gpt-")
    parts = r.split("-")
    ver = parts[0]
    qual = " ".join(p.title() for p in parts[1:])
    return f"GPT-{ver}" + (f" {qual}" if qual else "")

def label_gemini(m):
    r = m.removeprefix("gemini-")
    parts = r.split("-")
    ver = parts[0]
    qual = " ".join(p.title() for p in parts[1:])
    return f"Gemini {ver}" + (f" {qual}" if qual else "")

def label_grok(m):
    r = m.removeprefix("grok-")
    parts = r.split("-")
    vp, qp = [], []
    for p in parts:
        (vp if not qp and p.isdigit() else qp).append(p)
    ver = ".".join(vp)
    qual = " ".join(p.title() for p in qp)
    return f"Grok {ver}" + (f" {qual}" if qual else "")

LABELLERS = [
    ("claude-", label_claude),
    ("gpt-",    label_gpt),
    ("gemini-", label_gemini),
    ("grok-",   label_grok),
]

def make_label(m):
    for prefix, fn in LABELLERS:
        if m.startswith(prefix):
            return fn(m)
    return m

# ── Filtering ─────────────────────────────────────────────────────

SKIP_BARE = {"gpt", "grok", "sonnet", "opus", "haiku"}
SKIP_RE = re.compile(
    r"-(beta|preview|exp|live|base)(-|$)"
    r"|-(specific|customtools)$"
    r"|-v\d+$"
)
OLD_CLAUDE = re.compile(r"^claude-(3|2|instant|\d+-)")

def is_redundant(m, pool):
    # dated: model-YYYYMMDD(-v1)
    b = re.sub(r"-\d{8}(-v\d+)?$", "", m)
    if b != m and b in pool:
        return True
    # short numeric suffix: model-001
    b = re.sub(r"-\d{3}$", "", m)
    if b != m and b in pool:
        return True
    # -0 minor alias (claude-opus-4-0 when claude-opus-4 exists)
    if m.endswith("-0") and m[:-2] in pool:
        return True
    # -latest alias
    if m.endswith("-latest") and m.removesuffix("-latest") in pool:
        return True
    return False

def curate(models, prefixes):
    if not prefixes:
        return []
    keep = []
    for m in models:
        if m in SKIP_BARE:
            continue
        if not any(m.startswith(p) for p in prefixes):
            continue
        if OLD_CLAUDE.match(m):
            continue
        # New-style claude-{tier}-{major}: skip major < 4
        if m.startswith("claude-"):
            parts = m.removeprefix("claude-").split("-")
            if (len(parts) >= 2
                    and parts[0] in ("haiku", "sonnet", "opus")
                    and parts[1].isdigit()
                    and int(parts[1]) < 4):
                continue
        # GPT hyphen variants: prefer dot form (gpt-5-4 → gpt-5.4)
        if m.startswith("gpt-"):
            dot = re.sub(r"^(gpt-\d+)-(\d+)", r"\1.\2", m)
            if dot != m and dot in models:
                continue
        if SKIP_RE.search(m):
            continue
        if is_redundant(m, models):
            continue
        keep.append(m)
    return keep

# ── Sorting ───────────────────────────────────────────────────────

CLAUDE_TIER = {"opus": 0, "sonnet": 1, "haiku": 2}

def sort_models(models, primary_prefix):
    if primary_prefix == "claude-":
        def key(m):
            r = re.sub(r"-\d{8}$", "", m.removeprefix("claude-"))
            parts = r.split("-")
            t = CLAUDE_TIER.get(parts[0], 99)
            nums = [-int(x) for x in parts[1:] if x.isdigit()]
            if not nums:
                nums = [1]          # bare alias (e.g. "claude-haiku") → last in tier
            nums.append(0)          # pad so opus-4 (-4,0) sorts after opus-4-6 (-4,-6,0)
            return (t,) + tuple(nums)
        return sorted(models, key=key)
    return sorted(models, reverse=True)

# ── Agent mapping ─────────────────────────────────────────────────
# (agent_name, cli_sources, allowed_prefixes, primary_prefix)

AGENTS = [
    ("claude",  ["claude"],          ["claude-"], "claude-"),
    ("cline",   [],                  [],          ""),
    ("codex",   ["codex"],           ["gpt-"],    "gpt-"),
    ("copilot", ["copilot"],         ["gpt-", "claude-"], "gpt-"),
    ("gemini",  ["gemini"],          ["gemini-"], "gemini-"),
    ("grok",    ["grok"],            ["grok-"],   "grok-"),
    ("junie",   ["junie", "claude"], ["claude-"], "claude-"),
    ("xai",     ["grok"],            ["grok-"],   "grok-"),
]

result = {}
for agent, sources, prefixes, primary in AGENTS:
    pool = set()
    for src in sources:
        pool |= read_models(src)
    models = curate(pool, prefixes)
    models = sort_models(models, primary)
    result[agent] = [[m, make_label(m)] for m in models]

with open(out_path, "w") as f:
    json.dump(result, f, indent=2)
    f.write("\n")

print(f"wrote {out_path}", file=sys.stderr)
CURATE