# clay-knowledge — prototype

A fast, searchable, modern web rebuild of the digitalfire ceramics reference,
served from an **intermediate representation (IR)** extracted from the archived
site. Built with **axum + Leptos (SSR + hydration)** and **tantivy** full-text search.

```
digitalfire HTML  ──clay-extract──▶  IR (pages.jsonl)  ──clay-app──▶  fast pages + search
   (from the                          clean body_html,                  axum + Leptos SSR,
    1.8 GB archive)                   text, links, meta                 tantivy index
```

## Quick start

```bash
cd prototype

# 1. Build the IR from the archived site (run once; ~6,500 pages, a few seconds)
just extract
#   └─ cargo run -p clay-extract --release -- ../data/digitalfire-archive data/ir

# 2. Run the dev server (compiles wasm on first run) → http://127.0.0.1:8788
just dev
#   └─ cargo leptos watch
```

Requires `cargo-leptos` (`cargo install cargo-leptos`) and the `wasm32-unknown-unknown`
target. The archived site must be extracted at `../data/digitalfire-archive`
(the 1.8 GB `digitalfire-archive.zip` from archive.org, unzipped).

## Static site (GitHub Pages)

The Leptos dev server is for development; the **deployable artifact is a fully
static site** — no backend needed. Pipeline:

```
ground truth ─clay-extract→ data/ir ─clay-images→ data/web ─clay-ssg→ site/ ─pagefind→ search
              (HTML→IR)              (resize+rename+        (static HTML)   (client-side index)
                                      remap refs)
```

```bash
just static          # full: extract -> images -> ssg -> pagefind  (needs the archive)
just site            # rebuild site/ from the committed data/web (fast; no archive needed)
just serve-static    # preview at http://127.0.0.1:8790
```

- **`data/web/`** (committed) holds the web-ready content: images resized to ≤1200px
  and renamed `N.ext` (640 MB → 272 MB), with an `img_map.json` (new ↔ old) and the
  IR with every image reference remapped.
- **`clay-ssg`** renders one `index.html` per route (clean URLs), collection pages,
  the home page, and ~2,500 alias redirect stubs (old numeric-id URLs → canonical
  slugs), plus a Pagefind search box. Output is self-contained static HTML/CSS/img.
- **Pagefind** builds a chunked client-side search index (no server).
- Deploy: [`.github/workflows/pages.yml`](../.github/workflows/pages.yml) builds the
  site (with `BASE_URL=/<repo>/` for a project page) and publishes it to GitHub Pages.
  `site/` is git-ignored (regenerated in CI / locally); `data/web/` is the tracked input.

## Layout

| Path | What |
| --- | --- |
| `crates/ir` | Shared IR types (`Page`, `Link`, `CollectionInfo`) — serde structs |
| `crates/extract` | `clay-extract` bin: digitalfire HTML → `data/ir/pages.jsonl` |
| `app` | `clay-app`: the axum + Leptos SSR/hydrate site |
| `app/src/state.rs` | (ssr) loads the IR, builds the tantivy index, holds app state |
| `app/src/search.rs` | (ssr) tantivy schema, indexing, ranked query + snippets |
| `app/src/app.rs` | Leptos components, routes, and `#[server]` functions |
| `app/style/main.scss` | Modern clean theme (sidebar + sticky search + content) |

## How the IR is produced

`clay-extract` is the conversion engine. For each archived page it:

1. **Picks the main content container** (the highest-text element that is not
   inside `<nav>`), then keeps only its content child-blocks — **dropping
   link-dense navigation** (the oxide/material "pickers" and mega-menus) that
   otherwise dominate digitalfire pages.
2. **Sanitizes** with `ammonia` (drops scripts/nav/forms and all Bootstrap
   classes/ids, keeps semantic tags + tables) so the app's own CSS controls the look.
3. **Rewrites links**: `../glossary/frit.html`, same-directory `b2o3.html`, and
   absolute `digitalfire.com/...` URLs all become clean app routes
   (`/glossary/frit`, `/oxide/b2o3`) via a slug map built in a first pass.
   Image `src`s are rewritten to the local `/media/...` route.
4. Derives a **plain-text field** for the search index, a short **summary**, and
   a handful of **related** internal links, plus the **source_url** for provenance.

The app loads `pages.jsonl` at startup, builds an **in-memory tantivy index**
(title boosted, prefix-matched last term for as-you-type), and serves:

- `/` — collections overview
- `/:collection` — list (e.g. `/oxide`)
- `/:collection/:slug` — a rendered page (e.g. `/oxide/al2o3`)
- `/search?q=` — full search results
- instant search-as-you-type in the top bar (reactive Leptos + `#[server]` fn)
- `/media/*` — archived images served from the extracted ZIP

## Source assessment (which inputs were used, and where they fall short)

Three candidate sources were evaluated:

| Source | Has | Falls short |
| --- | --- | --- |
| **archive.org `digitalfire-archive.zip`** (1.8 GB, Akil Harris) — **used as corpus** | 9,495 HTML pages + 9,480 `.txt` + 3,147 images + 56 videos, organized by collection | Missing `typecode`; ~1,800 fewer pages than the sitemap (mostly `url/` redirect records + some pictures); raw boilerplate; no per-page provenance/timestamp |
| **`github.com/millandr121/digitalfire`** | Real structured JSON (`materials.json` 2 MB, oxides/recipes/minerals/temperatures) + a glaze-calculator + Wayback-fill pipeline | Only the *numeric chemistry core* — no glossary/article/hazard/trouble prose; React (not Rust); brand-new, 0 stars |
| **our Wayback per-page mirror** (`data/archive/`) | Clean `id_` HTML + provenance sidecars | Slower to collect; partial at prototype time |

The prototype builds its IR from the **ZIP** (most complete, includes images,
no extra load on anyone). The millandr121 structured JSON is the natural input
for the *chemistry-aware* features (oxide-analysis tables, UMF) — see STATUS.

## Scope & known limitations (prototype)

- **Knowledge-core only**: material, oxide, glossary, article, recipe, mineral,
  hazard, trouble, test, property, temperature, schedule (~6,500 pages). Directory
  listings (schools/stores/consultants/url) and the picture/video collections are
  excluded by design.
- Content extraction is heuristic; a few pages may keep a stray breadcrumb or
  drop an unusual section. Tables and prose come through cleanly in spot checks.
- Search is in-memory (rebuilt at startup); fine for ~4k pages.
- **Dedup (done):** slug + numeric-id duplicates are merged by content hash
  (6,571 → 4,062 canonical); numeric IDs alias to the human-readable slug and
  resolve transparently. 3 hazard pages still keep a footer (long-tail layout).
- **Chemistry (done):** typed panels sourced from `prototype/data/chem/` (millandr121
  dataset, attributed): **1,485 material** pages show an oxide-analysis + Unity (UMF)
  table plus an inline **UMF Stull-point SVG** (SiO₂ vs Al₂O₃); **73 oxide** pages show
  a reference-property table (expansion, frit softening point, M.O.R.). Coverage is
  ~92% of materials that *have* an analysis upstream — the rest (organics, fibers,
  waxes) legitimately have none. Minerals had empty analyses upstream.
- **Glaze Stull chart (done):** a Rust port of the Seger UMF engine computes each
  recipe's glaze unity formula (flux RO+R2O → 1.0); **86 recipe** pages plot it on a
  labelled **Stull chart** (glossy/matte/crazing zones, ≈cone 10). Multi-variation
  recipe records are split to the first variation. Try `/recipe/bory1`.
- Extraction quality was QA'd adversarially against the originals — see
  `docs/qa-report.md`. A Stull-chart view and broader chemistry are still open.
