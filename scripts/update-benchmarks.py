#!/usr/bin/env python3
"""
AI Hub Benchmark Updater
========================
从公开排行榜抓取最新的 AI 模型评测数据，更新到 pricing.json。

数据来源:
  - Arena ELO:  https://arena.ai/leaderboard (原 lmarena.ai)
  - Aider:      https://aider.chat/docs/leaderboards/
  - SWE-bench:  https://www.swebench.com

用法:
  python3 update-benchmarks.py              # 更新所有评测数据
  python3 update-benchmarks.py --dry-run    # 只显示变更，不写入
  python3 update-benchmarks.py --add-missing # 自动添加排行榜上有但本地没有的模型
  python3 update-benchmarks.py --source arena  # 只更新 Arena 数据
  python3 update-benchmarks.py --source aider  # 只更新 Aider 数据

零依赖：仅使用 Python 标准库。
"""

import json
import os
import re
import sys
import urllib.request
import urllib.error
from datetime import datetime
from pathlib import Path
from typing import Optional, List, Dict

# ============================================================
# Config
# ============================================================

PRICING_PATH = Path(
    os.environ.get("AI_HUB_PRICING_PATH", "")
    or Path.home() / "Library" / "Application Support" / "ai-hub" / "pricing.json"
)

SOURCES = {
    "arena": {
        "url": "https://arena.ai/leaderboard",
        "field": "arena_score",
        "description": "Arena ELO",
    },
    "aider": {
        "url": "https://aider.chat/docs/leaderboards/",
        "field": "aider_polyglot",
        "description": "Aider Polyglot",
    },
    "swe": {
        "url": "https://www.swebench.com",
        "field": "swe_bench",
        "description": "SWE-bench Verified",
    },
}

# Model name aliases: map arena/aider names → local model_id
# These handle the common mismatches between leaderboard naming and our IDs
ALIASES = {
    # Anthropic
    "claude-opus-4-20250514": "claude-opus-4-6",
    "claude opus 4.6": "claude-opus-4-6",
    "claude-opus-4-6-thinking": "claude-opus-4-6",
    "claude-sonnet-4-20250514": "claude-sonnet-4-6",
    "claude sonnet 4.6": "claude-sonnet-4-6",
    "claude-3-7-sonnet": "claude-sonnet-4-6",
    "claude-3-5-sonnet-20241022": "claude-sonnet-4-6",
    "claude-3-5-haiku-20241022": "claude-haiku-4-5-20251001",
    "claude opus 4.5": "claude-opus-4-5",
    "claude-opus-4-5-20251101": "claude-opus-4-5",
    # OpenAI
    "gpt-5.4-high": "gpt-5.4",
    "gpt-5 (high)": "gpt-5.4",
    "gpt-5 (medium)": "gpt-5.2",
    "gpt-5.2-chat-latest": "gpt-5.2",
    "gpt-5.2-chat-latest-20260210": "gpt-5.2",
    "chatgpt-4o-latest": "gpt-4o",
    "gpt-4o-2024-08-06": "gpt-4o",
    "o3-2025-04-16": "o3",
    "o3 (high)": "o3",
    "o3-pro (high)": "o3-pro",
    "o4-mini (high)": "o4-mini",
    "o3-mini (high)": "o3",
    # Google
    "gemini-2.5-pro-preview-06-05": "gemini-2.5-pro",
    "gemini 2.5 pro preview 05-06": "gemini-2.5-pro",
    "gemini 2.5 pro preview 03-25": "gemini-2.5-pro",
    "gemini-2.5-flash-preview-05-20": "gemini-2.5-flash",
    "gemini-2.5-flash-preview-04-17": "gemini-2.5-flash",
    "gemini-2.0-flash-exp": "gemini-2.0-flash",
    "gemini-3.1-pro-preview": "gemini-3.1-pro",
    # DeepSeek
    "deepseek v3 (0324)": "deepseek-chat",
    "deepseek chat v3": "deepseek-chat",
    "deepseek-v3.2-exp (chat)": "deepseek-v3.2",
    "deepseek-v3.2-exp (reasoner)": "deepseek-v3.2",
    "deepseek r1": "deepseek-reasoner",
    "deepseek r1 (0528)": "deepseek-reasoner",
    # xAI
    "grok-4.20-beta1": "grok-4",
    "grok 3 beta": "grok-3",
    "grok-4 (high)": "grok-4",
    # Others
    "kimi k2": "kimi-k2",
    "kimi-k2.5-thinking": "kimi-k2.5",
    "kimi-k2.5-instant": "kimi-k2.5",
    "qwen3 235b a22b": "qwen3-235b",
    "qwen3.5-max-preview": "qwen3.5-max",
    "qwen3-max-preview": "qwen3-max",
    "llama 4 maverick": "llama-4-maverick",
    "codestral 25.01": "codestral-latest",
    "ernie-5.0-0110": "ernie-5.0",
    "dola-seed-2.0-pro": "dola-seed-2.0-pro",
    "glm-5.1": "glm-5.1",
    "glm-5": "glm-5",
}


# ============================================================
# Fetch helpers
# ============================================================

def fetch_html(url: str, timeout: int = 30) -> str:
    """Fetch a URL and return its HTML content."""
    req = urllib.request.Request(url, headers={
        "User-Agent": "AI-Hub-Benchmark-Updater/1.0",
        "Accept": "text/html,application/xhtml+xml",
    })
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            # Handle redirects
            return resp.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as e:
        # Try following redirect manually
        if e.code in (301, 302, 307, 308):
            new_url = e.headers.get("Location", "")
            if new_url:
                return fetch_html(new_url, timeout)
        print(f"  [WARN] HTTP {e.code} fetching {url}")
        return ""
    except Exception as e:
        print(f"  [WARN] Error fetching {url}: {e}")
        return ""


def normalize_name(name: str) -> str:
    """Normalize model name for matching."""
    return name.lower().strip().replace("_", "-").replace("  ", " ")


def resolve_model_id(raw_name: str, local_ids: set) -> Optional[str]:
    """Try to match a leaderboard model name to a local model_id."""
    norm = normalize_name(raw_name)

    # Direct match
    if norm in local_ids:
        return norm

    # Alias lookup
    if norm in ALIASES:
        return ALIASES[norm]

    # Partial match: check if any local id is contained in the name
    for lid in local_ids:
        if lid in norm or norm in lid:
            return lid

    return None


# ============================================================
# Scrapers
# ============================================================

def scrape_arena(html: str) -> Dict[str, int]:
    """Extract Arena ELO scores from the leaderboard HTML.

    Arena.ai renders a table where each <tr> row contains:
    - Model name in: class="max-w-full truncate">MODEL</span>
    - ELO score in: class="text-sm">NNNN</span>
    """
    results = {}

    # Split by table rows
    rows = html.split("<tr")
    for row in rows:
        # Find model name
        name_match = re.search(
            r'class="max-w-full truncate">([^<]+)</span>', row
        )
        if not name_match:
            name_match = re.search(
                r'font-mono[^>]*>([a-z][\w.\-]+(?:\s*\([^)]*\))?)</span>', row
            )
        if not name_match:
            continue

        name = name_match.group(1).strip()

        # Skip video/image/audio models
        skip_keywords = [
            "video", "image", "720p", "480p", "1080p", "imagine",
            "veo-", "sora-", "kling-", "wan2", "seedream", "hunyuan-image",
            "seedance", "vidu", "audio",
        ]
        if any(kw in name.lower() for kw in skip_keywords):
            continue

        # Find ELO score
        score_match = re.search(r'class="text-sm">(1[1-5]\d{2})</span>', row)
        if score_match:
            score = int(score_match.group(1))
            norm = normalize_name(name)
            # Only keep the FIRST (highest) score per model
            # The page has multiple sections (Overall, Code, Search etc.)
            # with decreasing relevance
            if norm not in results:
                results[norm] = score

    return results


def scrape_aider(html: str) -> Dict[str, float]:
    """Extract Aider polyglot scores.

    Aider renders a table/chart. We look for model names near percentage scores
    in various HTML structures.
    """
    results = {}

    # Clean to text, then parse
    text = re.sub(r'<[^>]+>', '\n', html)
    lines = text.split('\n')

    # Look for lines with model-like names followed by percentage
    for i, line in enumerate(lines):
        line = line.strip()
        # Check for percentage on current or nearby line
        pct_match = re.search(r'(\d{1,2}\.\d)\s*%', line)
        if pct_match:
            val = float(pct_match.group(1))
            if val < 1.0 or val > 100.0:
                continue
            # The model name might be on this line or a few lines before
            name = None
            # Check this line for a model name before the percentage
            name_match = re.search(
                r'([\w][\w.\-\s/]+(?:\([^)]*\))?)\s+\d{1,2}\.\d\s*%', line
            )
            if name_match:
                name = name_match.group(1).strip()
            else:
                # Look at previous lines for a model name
                for j in range(max(0, i - 3), i):
                    prev = lines[j].strip()
                    if len(prev) > 3 and re.search(
                        r'(?:claude|gpt|gemini|grok|o[34]|glm|qwen|kimi|deepseek|llama|mistral|codestral)',
                        prev, re.IGNORECASE
                    ):
                        name = prev
                        break

            if name:
                norm = normalize_name(name)
                if norm not in results or val > results[norm]:
                    results[norm] = val

    # Also try structured patterns
    for pat in [
        r'([\w][\w.\-\s/]+(?:\([^)]*\))?)\s*[\|]\s*(\d{1,2}\.\d)\s*%',
        r'"(?:model|name)":\s*"([^"]+)"[^}]*"(?:score|percent)":\s*(\d{1,2}\.\d)',
    ]:
        for name, score in re.findall(pat, html, re.IGNORECASE):
            val = float(score)
            if 1.0 <= val <= 100.0:
                norm = normalize_name(name.strip())
                if norm not in results or val > results[norm]:
                    results[norm] = val

    return results


def scrape_swe_bench(html: str) -> Dict[str, float]:
    """Extract SWE-bench verified resolve rate scores."""
    results = {}

    # SWE-bench shows resolve rates as percentages in tables
    text = re.sub(r'<[^>]+>', '\n', html)

    for pat in [
        r'([\w][\w.\-\s/]+(?:\([^)]*\))?)\s*[\|:]\s*(\d{1,2}\.\d)\s*%',
        r'"(?:model|name)":\s*"([^"]+)"[^}]*"(?:resolved?_?rate|score)":\s*(\d{1,2}\.\d)',
    ]:
        for name, score in re.findall(pat, text, re.IGNORECASE):
            val = float(score)
            if 5.0 <= val <= 100.0:
                norm = normalize_name(name.strip())
                if norm not in results or val > results[norm]:
                    results[norm] = val

    return results


# ============================================================
# Main logic
# ============================================================

def load_pricing() -> dict:
    if not PRICING_PATH.exists():
        print(f"[ERROR] pricing.json not found at {PRICING_PATH}")
        sys.exit(1)
    with open(PRICING_PATH, "r") as f:
        return json.load(f)


def save_pricing(data: dict):
    tmp_path = PRICING_PATH.with_suffix(".json.tmp")
    with open(tmp_path, "w") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
    tmp_path.rename(PRICING_PATH)


def update_benchmarks(
    dry_run: bool = False,
    add_missing: bool = False,
    sources: Optional[List[str]] = None,
):
    data = load_pricing()
    local_ids = {m["model_id"] for m in data["models"]}
    changes = []

    active_sources = sources or list(SOURCES.keys())

    for src_key in active_sources:
        src = SOURCES.get(src_key)
        if not src:
            print(f"[WARN] Unknown source: {src_key}")
            continue

        print(f"\n--- {src['description']} ({src['url']}) ---")
        html = fetch_html(src["url"])
        if not html:
            print(f"  [SKIP] No data fetched")
            continue

        # Scrape
        if src_key == "arena":
            raw_scores = scrape_arena(html)
        elif src_key == "aider":
            raw_scores = scrape_aider(html)
        elif src_key == "swe":
            raw_scores = scrape_swe_bench(html)
        else:
            continue

        print(f"  Found {len(raw_scores)} entries from source")

        # Match and update — only apply the BEST (first/highest) score per model
        matched = 0
        unmatched = []
        best_scores = {}  # model_id → best score from this source

        for raw_name, score in raw_scores.items():
            model_id = resolve_model_id(raw_name, local_ids)
            if model_id:
                if isinstance(score, float):
                    score = round(score, 1)
                # Keep the highest score per model_id
                if model_id not in best_scores or score > best_scores[model_id]:
                    best_scores[model_id] = score
                matched += 1
            else:
                unmatched.append((raw_name, score))

        # Apply best scores
        for model_id, score in best_scores.items():
            for m in data["models"]:
                if m["model_id"] == model_id:
                    field = src["field"]
                    old_val = m.get(field, 0)
                    if old_val != score:
                        changes.append({
                            "model": m["model_name"],
                            "field": field,
                            "old": old_val,
                            "new": score,
                        })
                        m[field] = score
                    break

        print(f"  Matched: {matched}, Unmatched: {len(unmatched)}")
        if unmatched and len(unmatched) <= 20:
            for name, score in unmatched[:10]:
                print(f"    ? {name}: {score}")

    # Report
    print(f"\n{'='*50}")
    print(f"Changes: {len(changes)}")
    for c in changes:
        arrow = "+" if c["new"] > c["old"] else "-" if c["new"] < c["old"] else "="
        print(f"  {arrow} {c['model']}.{c['field']}: {c['old']} -> {c['new']}")

    if not changes:
        print("No changes detected.")
        return

    if dry_run:
        print(f"\n[DRY RUN] Would update {len(changes)} values. Use without --dry-run to apply.")
    else:
        data["last_updated"] = datetime.now().strftime("%Y-%m-%d %H:%M")
        save_pricing(data)
        print(f"\nSaved to {PRICING_PATH}")
        print(f"Updated at: {data['last_updated']}")


# ============================================================
# CLI
# ============================================================

def main():
    args = sys.argv[1:]
    dry_run = "--dry-run" in args
    add_missing = "--add-missing" in args
    sources = None

    if "--source" in args:
        idx = args.index("--source")
        if idx + 1 < len(args):
            sources = [args[idx + 1]]

    if "--help" in args or "-h" in args:
        print(__doc__)
        return

    print(f"AI Hub Benchmark Updater")
    print(f"Pricing file: {PRICING_PATH}")
    print(f"Mode: {'DRY RUN' if dry_run else 'LIVE UPDATE'}")
    if sources:
        print(f"Sources: {', '.join(sources)}")

    update_benchmarks(dry_run=dry_run, add_missing=add_missing, sources=sources)


if __name__ == "__main__":
    main()
