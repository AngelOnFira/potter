# STATUS — clay-knowledge (overnight 2026-06-17 → 06-18)

Honest handoff. Three things happened tonight: (1) the site was surveyed and is
being mirrored from Wayback, (2) a research report was produced, (3) a working
**axum + Leptos prototype** of a modern, searchable rebuild was built and verified.

---

## Update — Round 2 (2026-06-18): dedup · QA · extraction fixes · chemistry

All verified on the running server (http://127.0.0.1:8788).

- **Dedup** — digitalfire serves each page at both a slug and a numeric-id URL.
  Added a content-hash dedup pass to the extractor (canonical = human-readable
  slug; numeric IDs alias to it and resolve transparently; inline links remapped
  to canonical). **6,571 → 4,062 canonical pages**; counts now match the sitemap's
  true uniques (material **2,846** vs sitemap 2,843; glossary 362; oxide 109).
- **Adversarial QA** — ran a 60-page (5×12 collections) workflow: each page's
  extracted output critiqued against its original ZIP HTML, high-severity findings
  independently re-verified, synthesized into **`docs/qa-report.md`**
  (reviews in `docs/qa-reviews.json`). Avg fidelity **72/100**;
  content (tables/prose/recipes) was faithful — losses were boilerplate + link bugs.
- **Extraction fixes** (from the QA report) — strip the site-wide footer
  (PayPal/social/Ko-fi/privacy), the leftover "All Glossary" picker toggle, and the
  "Key phrases linking here" linkmaker block; fix link rewriting (host-based
  external detection killed the `/https://…` corruption; `../home.html` → `/`;
  correct same-dir vs root-relative resolution). Defect counts collapsed:

  | defect | before | after |
  |---|---:|---:|
  | footer (PayPal/ko-fi/privacy) | ~3,938 | 3 |
  | `#collapse` toggle | 997 | 3 |
  | `/https://` corrupted links | 3,839 | 0 |
  | `/<coll>/home` invented routes | 3,938 | 0 |

  (The QA "truncation" finding was a false positive from the sampler's 9,000-char
  cutoff — 0 real pages have unbalanced tables. 3 hazard pages keep a footer:
  long-tail layout edge.)
- **Chemistry layer** — wired millandr121's verified JSON into the IR. **1,485
  material pages**: typed **oxide-analysis + Unity (UMF) table** + an inline **UMF
  Stull-point SVG** (SiO₂ vs Al₂O₃). **73 oxide pages**: a **reference-property table**
  (linear expansion, frit softening point, M.O.R.). Matched by normalized
  name/slug/alt-name. Coverage is **~92% of materials that have an analysis upstream**;
  the unmatched (~1,361) are organics/additives (Nylon Fibers, Paraffin Wax…) that
  legitimately have no oxide analysis — not a matching failure. Minerals had empty
  analyses upstream. How millandr's data was sourced (clean Wayback/archive-first
  pipeline, provenance-tagged, chemistry-verified) is documented for trust.
- **Glaze Stull chart** — ported the Seger UMF engine to Rust (atomic weights →
  molecular weights → flux RO+R2O normalized to 1.0). **86 recipe pages** now show a
  **computed glaze unity formula** plotted on a labelled **Stull chart** (glossy /
  matte / crazing / under-fired zones, ≈cone 10). millandr's recipe records often
  concatenate several glaze variations into one ingredient list — we split at
  recurrences of the base material and compute the first variation, noting "N of M
  ingredients resolved". Verified chemically (e.g. `/recipe/bory1`: fluxes sum to 1.0).
  Turns the prototype from a reference into a glaze-chemistry *tool*.
- **Static site for GitHub Pages** — backend no longer required for deploy. New
  stages: **`clay-images`** (resize ≤1200px + rename to `N.ext` + remap refs;
  640 MB → 272 MB) → **`clay-ssg`** (one static `index.html` per route + collection/
  home pages + ~2,500 alias redirects; direct templating, shared CSS, no hydration
  cruft) → **Pagefind** (client-side search; 4,075 pages indexed). `site/` is fully
  static; verified locally (routes 200, chem/Stull intact, images + search load,
  aliases redirect). Tracked input: `prototype/data/web/`; deploy via
  `.github/workflows/pages.yml` (`BASE_URL=/<repo>/`). **Whole site → GitHub Pages: yes.**

**Current pipeline:** `clay-extract` now also de-dupes, remaps links, and attaches
chemistry; run `just extract` then `just dev`. New inputs: `prototype/data/chem/`
(materials.json, minerals.json from millandr121).

---

---

## 1. Survey + archive (done)
- **11,327** sitemap URLs across 22 collections; **99.2%** already in the Wayback
  Machine; only **90** genuinely missing → `data/missing_from_wayback.json`.
- Wayback holds **41,695** unique digitalfire URLs (more than the live sitemap).
- Tools (stdlib Python, throttled, Wayback-only): `tools/map_site.py`,
  `tools/archive_wayback.py`. See top-level `README.md`.
- **Running:** bulk Wayback mirror → `data/archive/` (resumable; ~1,070 / 11,237
  pages at handoff). Restart/continue: `python3 tools/archive_wayback.py fetch --delay 2.0`.

## 2. Research (done)
- Full report: **`docs/research-report.md`** (Wayback mechanics playbook,
  preservation/modernization prior art, rights/legal analysis, existing efforts).
- Raw per-dimension findings: `docs/research-findings.json`.
- Key external facts I verified tonight:
  - **archive.org `digitalfire-archive`** — a real 1.8 GB full-site ZIP (22,179
    files), uploaded 2026-06-17 by Akil Harris. Downloaded to
    `data/digitalfire-archive.zip`, extracted to `data/digitalfire-archive/`.
  - **github.com/millandr121/digitalfire** — real structured JSON
    (`materials.json` 2 MB, oxides/recipes/minerals/temperatures) + a glaze
    calculator + a Wayback-fill pipeline.
  - Rights: the shutdown is a **rights/T&C problem**; a public verbatim mirror
    carries real copyright risk. You chose to proceed with the full mirror — this
    is on record in `docs/research-report.md §6`, not a blocker.

## 3. Prototype (done — builds, runs, smoke-tested) → `prototype/`

**A modern, fast, searchable rebuild.** Stack: **axum + Leptos 0.8 (SSR +
hydration)**, **tantivy** full-text search, a Rust **HTML→IR extractor**
(`scraper` + `ammonia`). All Rust, in `prototype/`.

```
digitalfire HTML (1.8 GB ZIP) ─clay-extract→ IR: data/ir/pages.jsonl (6,571 pages)
                              ─clay-app→ axum+Leptos SSR pages + tantivy search
```

**Verified working** (server on http://127.0.0.1:8788):
- Home with 12 collection tiles + counts (Materials 4528, Oxides 216, Glossary
  720, …); SSR + hydration assets (`/pkg/clay-app.wasm` 9.1 MB) serve correctly.
- Rendered pages with **data tables + cross-links**, e.g. `/oxide/al2o3`
  ("Aluminum Oxide", oxide-analysis table, Related chips, provenance footer).
- Search works: `/search?q=feldspar` → Feldspar / Potash Feldspar …; instant
  search-as-you-type in the top bar (reactive Leptos `#[server]` fn + tantivy).
- Archived images served locally from `/media/*`.

**Run it:**
```bash
cd prototype
just extract          # once: HTML → data/ir/pages.jsonl  (a few seconds)
just dev              # cargo leptos watch → http://127.0.0.1:8788
#   or: just serve    # release build + serve
```
(The dev server may still be running from tonight on :8788; if not, `just dev`.)

### Decisions I made (you said to)
- **Corpus = the 1.8 GB ZIP** (most complete, includes images, zero extra load).
- **Design = modern clean** (sidebar + sticky instant search + content), per your pick.
- **Scope = knowledge-core** (material, oxide, glossary, article, recipe, mineral,
  hazard, trouble, test, property, temperature, schedule).
- **Chemistry = page-search-first**; typed UMF/Stull deferred (see below).
- Architecture: cargo-leptos SSR+hydrate; in-memory tantivy; IR as JSONL with
  rewritten links + provenance. Pinned `wasm-bindgen 0.2.115` to match the toolchain CLI.

### Source assessment (you asked me to vet the other projects)
| Source | Has | Falls short |
| --- | --- | --- |
| **archive.org ZIP** (used) | 9,495 HTML + 9,480 txt + 3,147 images + 56 videos, by collection | no `typecode`; ~1,800 fewer than sitemap (mostly `url/` redirect records); raw boilerplate; no per-page provenance |
| **millandr121 repo** | real structured chemistry JSON + glaze calc + Wayback-fill pipeline | only the numeric chemistry core (no prose collections); React not Rust; brand-new (0★) |
| **our Wayback mirror** | clean `id_` HTML + provenance sidecars | slower; partial at prototype time |

### Known limitations / honest caveats
- **Duplicate pages**: digitalfire exposes slug + numeric-id URLs for the same
  page (e.g. `/glossary/200-mesh` and `/glossary/343`), so counts/search ~double
  in some collections. Dedup by content digest is a quick fix.
- Extraction is heuristic — occasional leading breadcrumb noise ("Materials for
  Ceramics …"); tables and prose come through cleanly in spot checks.
- Search index is rebuilt in memory at startup (fine at 6.5k pages).
- `typecode` collection isn't in the ZIP (recoverable from Wayback CDX if wanted).

### Suggested next steps
1. **Dedup** slug/id pages by content digest (halves noise; quick win).
2. **Chemistry features** from millandr121 JSON: typed oxide-analysis + UMF tables,
   Stull-chart view on material/oxide pages.
3. Tighten per-collection extraction (strip breadcrumbs); add `typecode`.
4. Decide scope expansion (pictures/videos; all collections) and the publish path
   (the rights questions in `docs/research-report.md §6`).
5. Coordinate with Akil Harris / millandr121 rather than duplicate (research §5).

---
*Nothing is committed (no commits on this repo yet). Everything above is on disk
under `clay-knowledge/`. Background: the Wayback mirror is the only long-running job.*
