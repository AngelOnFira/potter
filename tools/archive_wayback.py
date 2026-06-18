#!/usr/bin/env python3
"""
archive_wayback.py - Archive digitalfire.com *from the Wayback Machine only*.

This tool NEVER touches the live digitalfire.com origin. It talks only to
web.archive.org. Workflow:

  index   Enumerate every digitalfire.com capture the Wayback Machine holds
          (CDX Server API, paginated + cached) and reduce to the latest
          HTTP-200 snapshot per unique URL.  ->  data/wayback/cdx_index.jsonl

  plan    Cross-reference the sitemap manifest (data/urls.jsonl, from
          map_site.py) against the Wayback index. Produces:
            data/archive_plan.jsonl          URLs we CAN pull from Wayback
            data/missing_from_wayback.json   URLs Wayback does NOT have  <-- review me
            data/coverage_report.md          human-readable coverage table

  fetch   Download the latest archived snapshot of each planned URL via the
          Wayback "id_" raw-bytes endpoint, mirroring the path under
          data/archive/, with a .meta.json provenance sidecar. Resumable.

  all     index -> plan -> fetch

Why Wayback-only?
  digitalfire.com shuts down 2026-06-26 and its owner reported the live server
  is overloaded (we even saw 'Connection refused' during the survey). Wayback
  content persists past the shutdown, so the only time-critical artifact is the
  *missing* list - pages that will be lost forever unless caught before 06-26.

Politeness:
  The Internet Archive also rate-limits (CDX 429s are common). This tool is
  single-threaded, throttles every request, and backs off on 429/5xx.

Pure standard library. No third-party dependencies.

Examples:
  python3 tools/archive_wayback.py all
  python3 tools/archive_wayback.py index --delay 4
  python3 tools/archive_wayback.py plan
  python3 tools/archive_wayback.py fetch --limit 50 --prefix /oxide/
"""
from __future__ import annotations

import argparse
import gzip
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from collections import defaultdict

# --------------------------------------------------------------------------- #

CDX_ENDPOINT = "http://web.archive.org/cdx/search/cdx"
WB_RAW = "https://web.archive.org/web/{ts}id_/{url}"  # raw archived bytes
DEFAULT_TARGET_DOMAIN = "digitalfire.com"
DEFAULT_AS_OF = "20260625"  # prefer the latest capture at/just-before shutdown

DEFAULT_UA = (
    "clay-knowledge-archiver/0.1 (+digitalfire preservation; Wayback-only; "
    "polite/throttled; contact: forestkzanderson@gmail.com)"
)

HERE = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(HERE)
DATA = os.path.join(REPO_ROOT, "data")


# --------------------------------------------------------------------------- #
# Polite networking
# --------------------------------------------------------------------------- #

class RateLimiter:
    def __init__(self, min_interval: float):
        self.min_interval = min_interval
        self._last = 0.0

    def wait(self) -> None:
        now = time.monotonic()
        gap = now - self._last
        if self._last and gap < self.min_interval:
            time.sleep(self.min_interval - gap)
        self._last = time.monotonic()


def http_get(url: str, limiter: RateLimiter, ua: str,
             max_retries: int = 5, timeout: float = 90.0):
    """GET -> (status, bytes, final_url). Retries 429/5xx with backoff."""
    attempt = 0
    while True:
        attempt += 1
        limiter.wait()
        req = urllib.request.Request(url, headers={
            "User-Agent": ua,
            "Accept-Encoding": "gzip",
            "Accept": "*/*",
        })
        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                raw = resp.read()
                if resp.headers.get("Content-Encoding") == "gzip" or raw[:2] == b"\x1f\x8b":
                    try:
                        raw = gzip.decompress(raw)
                    except OSError:
                        pass
                return resp.status, raw, resp.geturl()
        except urllib.error.HTTPError as e:
            if e.code == 404:
                return 404, b"", url
            if e.code in (429, 500, 502, 503, 504) and attempt <= max_retries:
                ra = e.headers.get("Retry-After") if e.headers else None
                backoff = float(ra) if (ra and str(ra).isdigit()) else min(
                    120.0, limiter.min_interval * (2 ** (attempt - 1)))
                print(f"    HTTP {e.code} -> backoff {backoff:.0f}s "
                      f"(attempt {attempt}/{max_retries}) {url}", file=sys.stderr)
                time.sleep(backoff)
                continue
            raise
        except (urllib.error.URLError, TimeoutError, ConnectionError) as e:
            if attempt <= max_retries:
                backoff = min(120.0, limiter.min_interval * (2 ** (attempt - 1)))
                print(f"    net error {e} -> retry {backoff:.0f}s "
                      f"(attempt {attempt}/{max_retries})", file=sys.stderr)
                time.sleep(backoff)
                continue
            raise


# --------------------------------------------------------------------------- #
# URL normalization (for matching sitemap <-> CDX without full SURT)
# --------------------------------------------------------------------------- #

# digitalfire's sitemap is buggy: ~11,340 of its <loc> entries use the host
# "digitlfire.com" (missing the 'a'), which does not even resolve in DNS. The
# real, Wayback-archived host is "digitalfire.com". Canonicalize before any
# matching or fetching, or nothing will line up with the Wayback index.
HOST_FIXES = {"digitlfire.com": "digitalfire.com"}


def canonical_url(u: str) -> str:
    p = urllib.parse.urlparse(u)
    host = (p.netloc or "").lower()
    bare = host[4:] if host.startswith("www.") else host
    if bare in HOST_FIXES:
        host = HOST_FIXES[bare]
    path = re.sub(r"/{2,}", "/", p.path or "/")  # collapse accidental // in paths
    path = path.replace(" ", "+")  # digitalfire slugs use '+' for spaces (e.g. /oxide/free+sio2)
    return urllib.parse.urlunparse(
        (p.scheme or "https", host, path, p.params, p.query, ""))


def norm_key(u: str) -> str:
    p = urllib.parse.urlparse(canonical_url(u))
    host = (p.netloc or "").lower()
    if host.startswith("www."):
        host = host[4:]
    path = (p.path or "/").rstrip("/") or "/"
    q = ("?" + p.query) if p.query else ""
    return (host + path + q).lower()


# --------------------------------------------------------------------------- #
# index: enumerate Wayback captures via CDX
# --------------------------------------------------------------------------- #

def cdx_num_pages(domain: str, limiter, ua: str, page_size: int) -> int | None:
    qs = urllib.parse.urlencode({
        "url": domain, "matchType": "domain",
        "filter": "statuscode:200", "showNumPages": "true",
        "pageSize": str(page_size),
    })
    status, body, _ = http_get(f"{CDX_ENDPOINT}?{qs}", limiter, ua)
    txt = body.decode("utf-8", "replace").strip()
    try:
        return int(txt)
    except ValueError:
        print(f"    showNumPages returned non-int: {txt!r}", file=sys.stderr)
        return None


def cmd_index(args):
    domain = args.domain
    limiter = RateLimiter(args.delay)
    cdx_dir = os.path.join(DATA, "wayback", "cdx")
    os.makedirs(cdx_dir, exist_ok=True)

    fields = ["urlkey", "timestamp", "original", "mimetype", "statuscode", "digest"]
    base_params = {
        "url": domain, "matchType": "domain",
        "fl": ",".join(fields), "filter": "statuscode:200",
        "output": "json", "pageSize": str(args.page_size),
    }

    npages = cdx_num_pages(domain, limiter, args.user_agent, args.page_size)
    if npages is None:
        npages = args.max_pages  # fall back to a bounded scan
        print(f"    falling back to scanning up to {npages} pages", file=sys.stderr)
    print(f"[index] domain={domain}  pages={npages}  delay={args.delay}s", file=sys.stderr)

    # latest-200 capture per urlkey + a capture counter
    latest: dict[str, dict] = {}
    capcount: dict[str, int] = defaultdict(int)
    total_rows = 0

    for page in range(npages):
        cache = os.path.join(cdx_dir, f"page_{page:05d}.json")
        if os.path.exists(cache) and not args.refresh:
            body = open(cache, "rb").read()
        else:
            qs = urllib.parse.urlencode({**base_params, "page": str(page)})
            print(f"  [page {page+1}/{npages}] fetching CDX ...", file=sys.stderr)
            status, body, _ = http_get(f"{CDX_ENDPOINT}?{qs}", limiter, args.user_agent)
            open(cache, "wb").write(body)
        try:
            rows = json.loads(body.decode("utf-8", "replace") or "[]")
        except json.JSONDecodeError:
            print(f"    page {page} not JSON, skipping", file=sys.stderr)
            continue
        if not rows:
            continue
        header = rows[0]
        for r in rows[1:]:
            rec = dict(zip(header, r))
            total_rows += 1
            uk = rec.get("urlkey", "")
            capcount[uk] += 1
            ts = rec.get("timestamp", "")
            cur = latest.get(uk)
            if cur is None or ts > cur["timestamp"]:
                latest[uk] = rec

    out = os.path.join(DATA, "wayback", "cdx_index.jsonl")
    with open(out, "w") as f:
        for uk, rec in sorted(latest.items()):
            rec["captures"] = capcount[uk]
            f.write(json.dumps(rec) + "\n")

    print(f"[index] {total_rows} 200-captures -> {len(latest)} unique archived URLs",
          file=sys.stderr)
    print(f"[index] wrote {out}", file=sys.stderr)
    return 0


# --------------------------------------------------------------------------- #
# plan: sitemap manifest vs Wayback index -> archive plan + missing list
# --------------------------------------------------------------------------- #

def load_jsonl(path):
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                yield json.loads(line)


def cmd_plan(args):
    manifest = os.path.join(DATA, "urls.jsonl")
    index = os.path.join(DATA, "wayback", "cdx_index.jsonl")
    if not os.path.exists(manifest):
        sys.exit(f"missing {manifest} - run map_site.py first")
    if not os.path.exists(index):
        sys.exit(f"missing {index} - run `archive_wayback.py index` first")

    # Build lookup: normalized original URL -> wayback record
    wb_by_norm: dict[str, dict] = {}
    for rec in load_jsonl(index):
        wb_by_norm.setdefault(norm_key(rec["original"]), rec)

    plan_rows, missing = [], []
    per_cat = defaultdict(lambda: {"total": 0, "in_wayback": 0, "missing": 0})

    for site in load_jsonl(manifest):
        # canonicalize the buggy digitlfire.com -> digitalfire.com so the
        # emitted plan/missing lists contain real, fetchable URLs.
        url, cat = canonical_url(site["url"]), site.get("category", "?")
        per_cat[cat]["total"] += 1
        hit = wb_by_norm.get(norm_key(url))
        if hit:
            per_cat[cat]["in_wayback"] += 1
            plan_rows.append({
                "url": url, "category": cat,
                "wayback_original": hit["original"],   # exact archived URL (case-correct)
                "wayback_timestamp": hit["timestamp"],
                "wayback_mimetype": hit.get("mimetype"),
                "wayback_digest": hit.get("digest"),
                "wayback_captures": hit.get("captures"),
            })
        else:
            per_cat[cat]["missing"] += 1
            missing.append({
                "url": url, "category": cat,
                "lastmod": site.get("lastmod"),
                "sitemap": site.get("sitemap"),
            })

    with open(os.path.join(DATA, "archive_plan.jsonl"), "w") as f:
        for r in plan_rows:
            f.write(json.dumps(r) + "\n")

    total = sum(c["total"] for c in per_cat.values())
    in_wb = sum(c["in_wayback"] for c in per_cat.values())
    miss = sum(c["missing"] for c in per_cat.values())
    missing_doc = {
        "summary": {
            "sitemap_urls": total,
            "in_wayback": in_wb,
            "missing_from_wayback": miss,
            "coverage_pct": round(100 * in_wb / total, 1) if total else 0,
            "note": ("These URLs are in the digitalfire sitemap but have NO "
                     "HTTP-200 capture in the Wayback Machine. They will be lost "
                     "when the site shuts down on 2026-06-26 unless captured. "
                     "Review and decide whether to Save-Page-Now / fetch live."),
            "by_category": dict(sorted(per_cat.items())),
        },
        "missing": sorted(missing, key=lambda m: (m["category"], m["url"])),
    }
    with open(os.path.join(DATA, "missing_from_wayback.json"), "w") as f:
        json.dump(missing_doc, f, indent=2)

    # coverage report (markdown)
    lines = [
        "# Wayback coverage of digitalfire.com sitemap URLs",
        "",
        f"- **Sitemap URLs:** {total:,}",
        f"- **In Wayback (>=1 HTTP-200 capture):** {in_wb:,} ({missing_doc['summary']['coverage_pct']}%)",
        f"- **Missing from Wayback:** {miss:,}  -> `data/missing_from_wayback.json`",
        "",
        "| Collection | Total | In Wayback | Missing | Coverage |",
        "| --- | ---: | ---: | ---: | ---: |",
    ]
    for cat, c in sorted(per_cat.items(), key=lambda kv: -kv[1]["total"]):
        cov = round(100 * c["in_wayback"] / c["total"], 1) if c["total"] else 0
        lines.append(f"| {cat} | {c['total']:,} | {c['in_wayback']:,} | {c['missing']:,} | {cov}% |")
    with open(os.path.join(DATA, "coverage_report.md"), "w") as f:
        f.write("\n".join(lines) + "\n")

    print(f"[plan] {total} sitemap URLs: {in_wb} in Wayback, {miss} missing "
          f"({missing_doc['summary']['coverage_pct']}%)", file=sys.stderr)
    print("[plan] wrote data/archive_plan.jsonl, data/missing_from_wayback.json, "
          "data/coverage_report.md", file=sys.stderr)
    return 0


# --------------------------------------------------------------------------- #
# fetch: pull latest snapshot bytes from Wayback
# --------------------------------------------------------------------------- #

EXT_BY_MIME = {
    "text/html": ".html", "application/pdf": ".pdf", "image/jpeg": ".jpg",
    "image/png": ".png", "image/gif": ".gif", "text/plain": ".txt",
    "application/json": ".json", "text/css": ".css",
    "application/javascript": ".js", "image/webp": ".webp",
}


def url_to_path(url: str, mimetype: str | None) -> str:
    p = urllib.parse.urlparse(url)
    host = p.netloc.lower()
    path = p.path
    if not path or path.endswith("/"):
        path = path + "index"
    # sanitize each segment
    segs = [re.sub(r"[^A-Za-z0-9._\-]", "_", s) for s in path.split("/") if s != ""]
    rel = os.path.join(host, *segs) if segs else os.path.join(host, "index")
    if p.query:
        rel += "__" + re.sub(r"[^A-Za-z0-9._\-]", "_", p.query)[:80]
    # ensure a sensible extension
    base = os.path.basename(rel)
    if "." not in base:
        rel += EXT_BY_MIME.get((mimetype or "").split(";")[0].strip(), ".html")
    return rel


def cmd_fetch(args):
    plan_path = os.path.join(DATA, "archive_plan.jsonl")
    if not os.path.exists(plan_path):
        sys.exit(f"missing {plan_path} - run `archive_wayback.py plan` first")
    limiter = RateLimiter(args.delay)
    arc_dir = os.path.join(DATA, "archive")
    os.makedirs(arc_dir, exist_ok=True)
    log_path = os.path.join(arc_dir, "_fetch_log.jsonl")
    logf = open(log_path, "a")

    rows = [r for r in load_jsonl(plan_path)]
    if args.prefix:
        rows = [r for r in rows if urllib.parse.urlparse(r["url"]).path.startswith(args.prefix)]
    if args.limit:
        rows = rows[: args.limit]

    print(f"[fetch] {len(rows)} URLs  delay={args.delay}s  as_of={args.as_of}", file=sys.stderr)
    ok = skip = fail = 0
    for i, r in enumerate(rows, 1):
        url = r["url"]
        rel = url_to_path(url, r.get("wayback_mimetype"))
        dest = os.path.join(arc_dir, rel)
        meta_path = dest + ".meta.json"
        if os.path.exists(dest) and os.path.exists(meta_path) and not args.refresh:
            skip += 1
            continue
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        # Candidate (timestamp, url) pairs, most-reliable first:
        #  1. exact archived URL + its latest-200 timestamp (from CDX index)
        #  2. canonical URL near shutdown date (Wayback redirects to closest)
        #  3. canonical URL + the recorded timestamp
        candidates = []
        if r.get("wayback_timestamp") and r.get("wayback_original"):
            candidates.append((r["wayback_timestamp"], r["wayback_original"]))
        candidates.append((args.as_of, url))
        if r.get("wayback_timestamp"):
            candidates.append((r["wayback_timestamp"], url))
        for ts, turl in candidates:
            if not ts:
                continue
            wb_url = WB_RAW.format(ts=ts, url=turl)
            try:
                status, body, final = http_get(wb_url, limiter, args.user_agent)
            except Exception as e:
                print(f"  [{i}/{len(rows)}] ERROR {url}: {e}", file=sys.stderr)
                status, body, final = 0, b"", wb_url
            if status == 200 and body:
                open(dest, "wb").write(body)
                # extract the real snapshot timestamp from the redirected URL
                m = re.search(r"/web/(\d{14})", final)
                real_ts = m.group(1) if m else ts
                meta = {
                    "original_url": url, "category": r.get("category"),
                    "wayback_url": final, "snapshot_timestamp": real_ts,
                    "requested_timestamp": ts,
                    "mimetype": r.get("wayback_mimetype"),
                    "digest": r.get("wayback_digest"),
                    "bytes": len(body), "saved_path": os.path.relpath(dest, REPO_ROOT),
                }
                json.dump(meta, open(meta_path, "w"), indent=2)
                logf.write(json.dumps({"url": url, "ts": real_ts, "ok": True}) + "\n")
                logf.flush()
                ok += 1
                if i % 25 == 0 or i == len(rows):
                    print(f"  [{i}/{len(rows)}] ok={ok} skip={skip} fail={fail}", file=sys.stderr)
                break
        else:
            fail += 1
            logf.write(json.dumps({"url": url, "ok": False}) + "\n")
            logf.flush()
            print(f"  [{i}/{len(rows)}] FAILED {url}", file=sys.stderr)

    print(f"[fetch] done: ok={ok} skip={skip} fail={fail}  -> {arc_dir}", file=sys.stderr)
    logf.close()
    return 0


# --------------------------------------------------------------------------- #

def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--user-agent", default=DEFAULT_UA)

    pi = sub.add_parser("index", parents=[common], help="enumerate Wayback captures via CDX")
    pi.add_argument("--domain", default=DEFAULT_TARGET_DOMAIN)
    pi.add_argument("--delay", type=float, default=4.0, help="seconds between CDX requests")
    pi.add_argument("--page-size", type=int, default=5)
    pi.add_argument("--max-pages", type=int, default=200, help="cap if showNumPages unavailable")
    pi.add_argument("--refresh", action="store_true")
    pi.set_defaults(func=cmd_index)

    pp = sub.add_parser("plan", parents=[common], help="cross-ref sitemap vs Wayback index")
    pp.set_defaults(func=cmd_plan)

    pf = sub.add_parser("fetch", parents=[common], help="download snapshots from Wayback")
    pf.add_argument("--delay", type=float, default=2.5, help="seconds between Wayback fetches")
    pf.add_argument("--as-of", default=DEFAULT_AS_OF, help="target timestamp (prefer latest near it)")
    pf.add_argument("--limit", type=int, default=0, help="only first N (0 = all)")
    pf.add_argument("--prefix", default="", help="only URLs whose path starts with this")
    pf.add_argument("--refresh", action="store_true")
    pf.set_defaults(func=cmd_fetch)

    pa = sub.add_parser("all", parents=[common], help="index -> plan -> fetch")
    pa.add_argument("--domain", default=DEFAULT_TARGET_DOMAIN)
    pa.add_argument("--delay", type=float, default=3.0)
    pa.add_argument("--page-size", type=int, default=5)
    pa.add_argument("--max-pages", type=int, default=200)
    pa.add_argument("--as-of", default=DEFAULT_AS_OF)
    pa.add_argument("--limit", type=int, default=0)
    pa.add_argument("--prefix", default="")
    pa.add_argument("--refresh", action="store_true")

    def cmd_all(a):
        cmd_index(a)
        cmd_plan(a)
        cmd_fetch(a)
        return 0
    pa.set_defaults(func=cmd_all)

    args = ap.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
