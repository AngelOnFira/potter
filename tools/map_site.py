#!/usr/bin/env python3
"""
map_site.py - Survey & map digitalfire.com via its XML sitemaps.

Politely (heavily throttled) downloads the sitemap index and every sub-sitemap,
parses all URLs, categorizes them by collection, dedupes, and emits a survey:

  data/sitemaps/<category>.xml   raw cached sitemap (so we never re-fetch)
  data/urls.jsonl                one JSON record per unique URL
  data/urls.csv                  same, as CSV
  data/survey_summary.json       machine-readable counts + path analysis
  data/survey_summary.md         human-readable summary table

Why so polite?
  digitalfire.com announced a shutdown on 2026-06-26, its owner warned the
  live server is OVERLOADED, and robots.txt declares `Crawl-delay: 20`. This
  tool therefore:
    * runs single-threaded (never parallel against the origin),
    * honors the robots.txt Crawl-delay (default floor 20s) for any origin GET,
    * caches every fetched sitemap to disk and reuses it on re-runs,
    * identifies itself with a descriptive User-Agent + contact,
    * backs off (respecting Retry-After) on 429/5xx.

Pure Python standard library. No third-party dependencies.

Usage:
  python3 tools/map_site.py                 # full survey, honoring crawl-delay
  python3 tools/map_site.py --delay 25      # raise the per-request delay
  python3 tools/map_site.py --only oxide,material   # just those sitemaps
  python3 tools/map_site.py --refresh       # ignore cache, re-fetch sitemaps
"""
from __future__ import annotations

import argparse
import csv
import gzip
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from collections import Counter
from xml.etree import ElementTree as ET

# --------------------------------------------------------------------------- #
# Configuration / constants
# --------------------------------------------------------------------------- #

DEFAULT_INDEX = "https://digitalfire.com/sitemapindex.xml"
ROBOTS_URL = "https://digitalfire.com/robots.txt"
DEFAULT_DELAY_FLOOR = 20.0  # seconds; matches digitalfire robots.txt Crawl-delay
SITEMAP_NS = "{http://www.sitemaps.org/schemas/sitemap/0.9}"

DEFAULT_UA = (
    "clay-knowledge-archiver/0.1 (+digitalfire preservation survey; "
    "polite/throttled; contact: forestkzanderson@gmail.com)"
)

HERE = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(HERE)
DEFAULT_OUT = os.path.join(REPO_ROOT, "data")


# --------------------------------------------------------------------------- #
# Polite networking
# --------------------------------------------------------------------------- #

class RateLimiter:
    """Enforce a minimum interval between requests *per host*."""

    def __init__(self, min_interval: float):
        self.min_interval = min_interval
        self._last: dict[str, float] = {}

    def wait(self, host: str) -> None:
        now = time.monotonic()
        last = self._last.get(host, 0.0)
        gap = now - last
        if last and gap < self.min_interval:
            sleep_for = self.min_interval - gap
            print(f"    (throttle: sleeping {sleep_for:.1f}s before next {host} request)",
                  file=sys.stderr)
            time.sleep(sleep_for)
        self._last[host] = time.monotonic()


def polite_get(url: str, limiter: RateLimiter, ua: str,
               max_retries: int = 4, timeout: float = 60.0) -> bytes:
    """GET a URL with rate limiting, retries, and Retry-After-aware backoff."""
    host = urllib.parse.urlparse(url).netloc
    attempt = 0
    while True:
        attempt += 1
        limiter.wait(host)
        req = urllib.request.Request(url, headers={
            "User-Agent": ua,
            "Accept": "application/xml, text/xml, text/plain, */*",
        })
        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                data = resp.read()
            # Transparently handle gzip-compressed sitemaps (.xml.gz or magic bytes)
            if url.endswith(".gz") or data[:2] == b"\x1f\x8b":
                try:
                    data = gzip.decompress(data)
                except OSError:
                    pass
            return data
        except urllib.error.HTTPError as e:
            if e.code in (429, 500, 502, 503, 504) and attempt <= max_retries:
                retry_after = e.headers.get("Retry-After") if e.headers else None
                backoff = float(retry_after) if (retry_after and retry_after.isdigit()) \
                    else min(60.0, limiter.min_interval * attempt)
                print(f"    HTTP {e.code} on {url} -> backoff {backoff:.0f}s "
                      f"(attempt {attempt}/{max_retries})", file=sys.stderr)
                time.sleep(backoff)
                continue
            raise
        except (urllib.error.URLError, TimeoutError) as e:
            if attempt <= max_retries:
                backoff = min(60.0, limiter.min_interval * attempt)
                print(f"    network error on {url}: {e} -> retry in {backoff:.0f}s "
                      f"(attempt {attempt}/{max_retries})", file=sys.stderr)
                time.sleep(backoff)
                continue
            raise


# --------------------------------------------------------------------------- #
# robots.txt
# --------------------------------------------------------------------------- #

def parse_robots(text: str) -> dict:
    """Extract Crawl-delay, Disallow (for '*' and the most permissive set), Sitemaps."""
    crawl_delay = None
    disallow: list[str] = []
    sitemaps: list[str] = []
    current_agents: list[str] = []
    applies_to_star = False
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            current_agents = []
            applies_to_star = False
            continue
        if ":" not in line:
            continue
        field, _, value = line.partition(":")
        field = field.strip().lower()
        value = value.strip()
        if field == "user-agent":
            current_agents.append(value)
            if value == "*":
                applies_to_star = True
        elif field == "crawl-delay":
            try:
                cd = float(value)
                crawl_delay = cd if crawl_delay is None else max(crawl_delay, cd)
            except ValueError:
                pass
        elif field == "disallow" and applies_to_star and value:
            disallow.append(value)
        elif field == "sitemap":
            sitemaps.append(value)
    return {"crawl_delay": crawl_delay, "disallow": disallow, "sitemaps": sitemaps}


# --------------------------------------------------------------------------- #
# Sitemap parsing
# --------------------------------------------------------------------------- #

def _findtext(el, tag):
    child = el.find(SITEMAP_NS + tag)
    if child is None:
        child = el.find(tag)  # namespace-less fallback
    return child.text.strip() if (child is not None and child.text) else None


def parse_sitemap_index(data: bytes) -> list[dict]:
    root = ET.fromstring(data)
    out = []
    for sm in root.iter():
        if sm.tag.endswith("sitemap"):
            loc = _findtext(sm, "loc")
            if loc:
                out.append({"loc": loc, "lastmod": _findtext(sm, "lastmod")})
    return out


def parse_urlset(data: bytes) -> list[dict]:
    root = ET.fromstring(data)
    out = []
    for u in root.iter():
        if u.tag.endswith("}url") or u.tag == "url":
            loc = _findtext(u, "loc")
            if loc:
                out.append({"loc": loc, "lastmod": _findtext(u, "lastmod")})
    return out


def category_from_sitemap(loc: str) -> str:
    """sitemap-oxide.xml -> 'oxide'; sitemapmisc.xml -> 'misc'."""
    base = os.path.basename(urllib.parse.urlparse(loc).path)
    base = re.sub(r"\.xml(\.gz)?$", "", base, flags=re.I)
    base = re.sub(r"^sitemap", "", base, flags=re.I)
    base = base.lstrip("-_")
    return base or "index"


def normalize_sitemap_key(loc: str) -> str:
    """Collapse http/https + trailing slash so the http+https picture dupes merge."""
    p = urllib.parse.urlparse(loc)
    return (p.netloc.lower() + p.path.rstrip("/")).lower()


def is_disallowed(url: str, disallow: list[str]) -> bool:
    path = urllib.parse.urlparse(url).path
    return any(path.startswith(rule) for rule in disallow)


def is_http_url(s: str) -> bool:
    """True for a real http(s) URL. digitalfire's video sitemap is corrupt and
    emits bare timestamps in <loc> (e.g. '2024-01-31T00:00:00+00:00'); drop those."""
    p = urllib.parse.urlparse(s)
    return p.scheme in ("http", "https") and "." in (p.netloc or "")


# --------------------------------------------------------------------------- #
# Main survey
# --------------------------------------------------------------------------- #

def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--index", default=DEFAULT_INDEX, help="sitemap index URL")
    ap.add_argument("--out", default=DEFAULT_OUT, help="output directory (default: ./data)")
    ap.add_argument("--delay", type=float, default=None,
                    help="seconds between origin requests (default: max(robots crawl-delay, 20))")
    ap.add_argument("--user-agent", default=DEFAULT_UA)
    ap.add_argument("--only", default=None,
                    help="comma-separated category filter, e.g. 'oxide,material'")
    ap.add_argument("--refresh", action="store_true", help="ignore cached sitemaps, re-fetch")
    ap.add_argument("--no-robots", action="store_true", help="do not fetch/honor robots.txt")
    args = ap.parse_args()

    out_dir = os.path.abspath(args.out)
    sm_dir = os.path.join(out_dir, "sitemaps")
    os.makedirs(sm_dir, exist_ok=True)

    # Bootstrap limiter with the floor; tighten after reading robots.txt.
    limiter = RateLimiter(args.delay if args.delay is not None else DEFAULT_DELAY_FLOOR)

    robots = {"crawl_delay": None, "disallow": [], "sitemaps": []}
    if not args.no_robots:
        print(f"[1/4] Fetching robots.txt ...", file=sys.stderr)
        try:
            robots = parse_robots(polite_get(ROBOTS_URL, limiter, args.user_agent).decode("utf-8", "replace"))
            with open(os.path.join(out_dir, "robots.txt"), "w") as f:
                f.write(json.dumps(robots, indent=2))
        except Exception as e:
            print(f"    warning: could not fetch robots.txt: {e}", file=sys.stderr)

    # Effective delay = max(user/floor, robots crawl-delay)
    effective_delay = args.delay if args.delay is not None else DEFAULT_DELAY_FLOOR
    if robots.get("crawl_delay"):
        effective_delay = max(effective_delay, robots["crawl_delay"])
    limiter.min_interval = effective_delay
    print(f"    effective origin delay: {effective_delay:.0f}s  "
          f"(robots crawl-delay={robots.get('crawl_delay')})", file=sys.stderr)
    if robots.get("disallow"):
        print(f"    robots disallow: {robots['disallow']}", file=sys.stderr)

    # ----- sitemap index -----
    print(f"[2/4] Fetching sitemap index: {args.index}", file=sys.stderr)
    index_cache = os.path.join(sm_dir, "_index.xml")
    if os.path.exists(index_cache) and not args.refresh:
        index_data = open(index_cache, "rb").read()
        print("    (using cached index)", file=sys.stderr)
    else:
        index_data = polite_get(args.index, limiter, args.user_agent)
        open(index_cache, "wb").write(index_data)
    sub_sitemaps = parse_sitemap_index(index_data)

    # De-duplicate sub-sitemaps (the http+https picture entry collapses to one).
    seen_keys: set[str] = set()
    unique_sitemaps = []
    for sm in sub_sitemaps:
        key = normalize_sitemap_key(sm["loc"])
        if key in seen_keys:
            continue
        seen_keys.add(key)
        sm["category"] = category_from_sitemap(sm["loc"])
        # Prefer https form for actual fetching
        sm["fetch_loc"] = sm["loc"].replace("http://", "https://", 1)
        unique_sitemaps.append(sm)

    only = set(s.strip() for s in args.only.split(",")) if args.only else None
    if only:
        unique_sitemaps = [s for s in unique_sitemaps if s["category"] in only]

    print(f"    {len(sub_sitemaps)} sitemap entries -> {len(unique_sitemaps)} unique"
          + (f" (filtered to {sorted(only)})" if only else ""), file=sys.stderr)

    # ----- each sub-sitemap -----
    print(f"[3/4] Fetching {len(unique_sitemaps)} sub-sitemaps "
          f"(~{effective_delay:.0f}s apart; est. {len(unique_sitemaps)*effective_delay/60:.1f} min) ...",
          file=sys.stderr)

    all_records: list[dict] = []
    malformed_records: list[dict] = []
    per_category: list[dict] = []
    for i, sm in enumerate(unique_sitemaps, 1):
        cat = sm["category"]
        cache_path = os.path.join(sm_dir, f"{cat}.xml")
        if os.path.exists(cache_path) and not args.refresh:
            data = open(cache_path, "rb").read()
            cached = True
        else:
            print(f"  [{i}/{len(unique_sitemaps)}] {cat}: {sm['fetch_loc']}", file=sys.stderr)
            data = polite_get(sm["fetch_loc"], limiter, args.user_agent)
            open(cache_path, "wb").write(data)
            cached = False
        try:
            urls = parse_urlset(data)
        except ET.ParseError as e:
            print(f"    parse error in {cat}: {e}", file=sys.stderr)
            urls = []
        valid = [u for u in urls if is_http_url(u["loc"])]
        bad = [u for u in urls if not is_http_url(u["loc"])]
        for u in bad:
            malformed_records.append({"loc": u["loc"], "category": cat, "sitemap": sm["loc"]})
        for u in valid:
            all_records.append({
                "url": u["loc"],
                "category": cat,
                "lastmod": u["lastmod"],
                "sitemap": sm["loc"],
                "robots_disallowed": is_disallowed(u["loc"], robots.get("disallow", [])),
            })
        sections = Counter(
            (urllib.parse.urlparse(u["loc"]).path.strip("/").split("/", 1)[0] or "(root)")
            for u in valid
        )
        per_category.append({
            "category": cat,
            "sitemap_url": sm["loc"],
            "lastmod": sm.get("lastmod"),
            "url_count": len(valid),
            "malformed_loc_count": len(bad),
            "cached": cached,
            "path_sections": dict(sections.most_common(8)),
            "sample_urls": [u["loc"] for u in valid[:5]],
        })
        print(f"      -> {len(valid)} urls"
              + (f" (+{len(bad)} malformed dropped)" if bad else "")
              + (" (cached)" if cached else ""), file=sys.stderr)

    # ----- dedupe + analysis -----
    print(f"[4/4] Deduplicating & writing outputs ...", file=sys.stderr)
    unique: dict[str, dict] = {}
    for rec in all_records:
        unique.setdefault(rec["url"], rec)
    unique_records = list(unique.values())

    section_hist = Counter(
        (urllib.parse.urlparse(r["url"]).path.strip("/").split("/", 1)[0] or "(root)")
        for r in unique_records
    )
    disallowed_count = sum(1 for r in unique_records if r["robots_disallowed"])

    # urls.jsonl
    with open(os.path.join(out_dir, "urls.jsonl"), "w") as f:
        for r in sorted(unique_records, key=lambda x: (x["category"], x["url"])):
            f.write(json.dumps(r) + "\n")
    # urls.csv
    with open(os.path.join(out_dir, "urls.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=["url", "category", "lastmod", "sitemap", "robots_disallowed"])
        w.writeheader()
        for r in sorted(unique_records, key=lambda x: (x["category"], x["url"])):
            w.writerow(r)

    summary = {
        "index_url": args.index,
        "effective_origin_delay_s": effective_delay,
        "robots": robots,
        "totals": {
            "sitemaps_listed": len(sub_sitemaps),
            "sitemaps_unique": len(unique_sitemaps),
            "urls_total_with_dupes": len(all_records),
            "urls_unique": len(unique_records),
            "urls_robots_disallowed": disallowed_count,
            "malformed_loc_total": len(malformed_records),
        },
        "per_category": sorted(per_category, key=lambda c: -c["url_count"]),
        "path_section_histogram": dict(section_hist.most_common()),
    }
    with open(os.path.join(out_dir, "survey_summary.json"), "w") as f:
        json.dump(summary, f, indent=2)
    if malformed_records:
        with open(os.path.join(out_dir, "malformed_locs.json"), "w") as f:
            json.dump(malformed_records, f, indent=2)

    # Human-readable markdown summary
    write_markdown_summary(os.path.join(out_dir, "survey_summary.md"), summary)

    # ----- console report -----
    print("\n" + "=" * 64)
    print("DIGITALFIRE.COM SITEMAP SURVEY")
    print("=" * 64)
    print(f"Sitemaps:        {len(unique_sitemaps)} unique ({len(sub_sitemaps)} listed)")
    print(f"URLs (total):    {len(all_records)}")
    print(f"URLs (unique):   {len(unique_records)}")
    print(f"Robots-blocked:  {disallowed_count}")
    print(f"Origin delay:    {effective_delay:.0f}s\n")
    print(f"{'CATEGORY':<16}{'URLS':>9}   SAMPLE PATH SECTIONS")
    print("-" * 64)
    for c in summary["per_category"]:
        secs = ", ".join(f"{k}({v})" for k, v in list(c["path_sections"].items())[:3])
        print(f"{c['category']:<16}{c['url_count']:>9}   {secs}")
    print("-" * 64)
    print(f"{'TOTAL (unique)':<16}{len(unique_records):>9}")
    print(f"\nWrote: {out_dir}/{{urls.jsonl,urls.csv,survey_summary.json,survey_summary.md}}")
    return 0


def write_markdown_summary(path: str, s: dict) -> None:
    t = s["totals"]
    lines = [
        "# digitalfire.com — sitemap survey",
        "",
        f"- **Unique sitemaps:** {t['sitemaps_unique']} ({t['sitemaps_listed']} listed)",
        f"- **Unique URLs:** {t['urls_unique']:,} ({t['urls_total_with_dupes']:,} incl. cross-sitemap dupes)",
        f"- **Robots-disallowed URLs:** {t['urls_robots_disallowed']:,}",
        f"- **Origin crawl-delay honored:** {s['effective_origin_delay_s']:.0f}s",
        "",
        "## URLs by collection",
        "",
        "| Collection | URLs | Sample path sections |",
        "| --- | ---: | --- |",
    ]
    for c in s["per_category"]:
        secs = ", ".join(f"`{k}`×{v}" for k, v in list(c["path_sections"].items())[:4])
        lines.append(f"| {c['category']} | {c['url_count']:,} | {secs} |")
    lines += [
        "",
        "## Sample URLs per collection",
        "",
    ]
    for c in s["per_category"]:
        lines.append(f"### {c['category']} ({c['url_count']:,})")
        for u in c["sample_urls"]:
            lines.append(f"- {u}")
        lines.append("")
    with open(path, "w") as f:
        f.write("\n".join(lines))


if __name__ == "__main__":
    sys.exit(main())
