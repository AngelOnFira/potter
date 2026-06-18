#!/usr/bin/env python3
"""
qa_sample.py - build a side-by-side QA sample for adversarial extraction review.

For a stratified + "suspicious" sample of pages, writes one comparison file per
page containing OUR extracted output next to the ORIGINAL digitalfire HTML, plus a
manifest the QA workflow consumes.

  data/qa/<id>.txt        OUR output  ||  ORIGINAL html  (for one page)
  data/qa/samples.json    [{id, path, title, collection, file}, ...]

Usage: python3 tools/qa_sample.py [per_collection]   (default 5)
"""
import json
import os
import re
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
IR = os.path.join(REPO, "prototype", "data", "ir", "pages.jsonl")
ARCHIVE = os.path.join(REPO, "data", "digitalfire-archive")
QA = os.path.join(REPO, "data", "qa")

SCRIPT_RE = re.compile(r"(?is)<(script|style|noscript)\b.*?</\1>")
TAG_RE = re.compile(r"<[^>]+>")
WS_RE = re.compile(r"[ \t]+")


def orig_path(source_url, collection):
    # source_url = https://digitalfire.com/<collection>/<raw_stem>
    rest = source_url.split("digitalfire.com/", 1)[-1]
    return os.path.join(ARCHIVE, rest + ".html")


def suspicion(p):
    """Higher = more likely to be poorly extracted."""
    s = 0
    text = p.get("text", "")
    if len(text) < 250:
        s += 3
    body = p.get("body_html", "")
    # leading breadcrumb noise
    if re.match(r"\s*(All |Materials for Ceramics|<h1>)?\s*(All\b|Materials for Ceramics)", text):
        s += 2
    if text[:60].lower().startswith(("all ", "materials for ceramics")):
        s += 2
    # chemistry collections that lost their table
    if p["collection"] in ("material", "oxide", "mineral", "recipe") and "<table" not in body:
        s += 3
    if "Key phrases linking here" in text:
        s += 1
    return s


def main():
    per = int(sys.argv[1]) if len(sys.argv) > 1 else 5
    os.makedirs(QA, exist_ok=True)
    pages = [json.loads(l) for l in open(IR, encoding="utf-8") if l.strip()]

    by_col = {}
    for p in pages:
        by_col.setdefault(p["collection"], []).append(p)

    selected = []
    for col, ps in sorted(by_col.items()):
        ps_sorted = sorted(ps, key=lambda p: p["path"])
        # 2 most suspicious + the rest spread evenly across the collection
        susp = sorted(ps, key=suspicion, reverse=True)[: max(2, per // 2)]
        chosen = {p["path"]: p for p in susp}
        stride = max(1, len(ps_sorted) // max(1, per))
        for i in range(0, len(ps_sorted), stride):
            if len(chosen) >= per:
                break
            chosen.setdefault(ps_sorted[i]["path"], ps_sorted[i])
        selected.extend(list(chosen.values())[:per])

    manifest = []
    for p in selected:
        pid = p["path"].strip("/").replace("/", "__")
        ofile = orig_path(p["source_url"], p["collection"])
        original = ""
        if os.path.exists(ofile):
            html = open(ofile, encoding="utf-8", errors="replace").read()
            html = SCRIPT_RE.sub("", html)
            original = html[:16000]
        else:
            original = f"(original file not found: {ofile})"
        our = p.get("body_html", "")[:9000]
        text_preview = WS_RE.sub(" ", TAG_RE.sub(" ", our))[:400]
        content = (
            f"PAGE: {p['path']}\nCOLLECTION: {p['collection']}\nTITLE: {p['title']}\n"
            f"SOURCE: {p['source_url']}\n\n"
            f"===== OUR EXTRACTED OUTPUT (body_html we render; {len(p.get('body_html',''))} chars total) =====\n"
            f"{our}\n\n"
            f"----- (plain-text preview of our output) -----\n{text_preview}\n\n"
            f"===== ORIGINAL DIGITALFIRE HTML (scripts/styles stripped, truncated) =====\n"
            f"{original}\n"
        )
        fpath = os.path.join(QA, f"{pid}.txt")
        open(fpath, "w", encoding="utf-8").write(content)
        manifest.append({
            "id": pid,
            "path": p["path"],
            "title": p["title"],
            "collection": p["collection"],
            "file": fpath,
        })

    json.dump(manifest, open(os.path.join(QA, "samples.json"), "w"), indent=2)
    print(f"wrote {len(manifest)} QA comparison files to {QA}")
    from collections import Counter
    print("by collection:", dict(Counter(m["collection"] for m in manifest)))


if __name__ == "__main__":
    main()
