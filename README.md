# clay-knowledge

Preserving **[digitalfire.com](https://digitalfire.com)** — Tony Hansen's 35-year
ceramics & glaze-chemistry reference — into a modern, open, searchable archive
before the site shuts down on **2026-06-26**.

> Digitalfire announced it will go offline on June 26, 2026. The owner reported
> the live server is overloaded ("the internet archive has everything... please
> be patient, I'm setting up throttling") and asked people to stop bulk-downloading.
> This project therefore preserves the content **from the Wayback Machine**, never
> by hammering the live origin.

## Status (2026-06-17)

| | |
| --- | --- |
| Sitemap URLs mapped | **11,327** across 22 collections |
| Already in the Wayback Machine | **11,237 (99.2%)** |
| Missing from Wayback (at risk) | **90** → [`data/missing_from_wayback.json`](data/missing_from_wayback.json) |
| Unique digitalfire URLs Wayback holds | **41,695** (3.7× the current sitemap) |

**Bottom line:** the irreplaceable knowledge (materials, oxides, glossary,
articles, recipes, minerals, hazards) is **~100% already safe in Wayback**, which
persists past the shutdown. The only time-critical work before June 26 is the
short [missing list](data/missing_from_wayback.json) — mostly recently-added
pages and external link-records.

## What digitalfire is

A structured, heavily cross-linked **reference wiki** for ceramics. Each
collection is a typed table of pages:

| Collection | Pages | What it is |
| --- | ---: | --- |
| `material` | 2,843 | **The core** — clay/glaze materials, each with oxide analysis, properties, suppliers |
| `picture` | 2,823 | Figures/photos illustrating tests, defects, materials |
| `url` | 3,443 | External reference/citation records (link database) |
| `glossary` | 362 | Glaze-chemistry term essays (the conceptual backbone) |
| `typecode` | 267 | Material/property type codes |
| `article` | 112 | Tony Hansen's long-form authored articles |
| `oxide` | 109 | The chemical oxides (SiO₂, Al₂O₃, …) glazes are built from |
| `mineral` | 108 | Mineral reference pages |
| `test` | 100 | Standardized ceramic test procedures |
| `hazard` | 92 | Material safety / toxicity pages |
| `recipe` | 87 | Glaze/body recipes (small — most live behind Insight-Live) |
| `temperature` / `schedule` | 74 / 29 | Firing temperatures & kiln firing schedules |
| `trouble` / `property` | 30 / 14 | Glaze-defect troubleshooting & measurable properties |
| `schools` / `stores` / `consultants` | 611 / 106 / 74 | Directory listings (low unique-knowledge value) |
| `project` / `video` / `misc` / `potterytony` | 54 / 1* / 3 / 1 | Misc. (\*video sitemap is corrupt at source — see findings) |

URLs follow a clean `/{collection}/{id-or-slug}` pattern (e.g.
`/oxide/sio2`, `/material/2287`, `/glossary/200+mesh`), and every collection has
a `/{collection}/list` index — ideal for structured extraction later.

## The pipeline

Three throttled, resumable, **standard-library-only** Python tools in `tools/`:

```
1. map_site.py         -> survey the sitemaps        (touches digitalfire ONCE per
   (live, throttled)      data/urls.jsonl, *.csv      sitemap, honoring 20s crawl-delay)
                          data/survey_summary.{json,md}

2. archive_wayback.py index   -> enumerate Wayback via CDX   (web.archive.org only)
                                 data/wayback/cdx_index.jsonl

3. archive_wayback.py plan    -> coverage + the at-risk list (local, instant)
                                 data/archive_plan.jsonl
                                 data/missing_from_wayback.json   <-- review me
                                 data/coverage_report.md

4. archive_wayback.py fetch   -> download snapshots from Wayback (web.archive.org only)
                                 data/archive/digitalfire.com/<path>  + .meta.json sidecars
```

### Run it

```bash
# 1. Map the live site (≈8 min; honors robots.txt Crawl-delay: 20s)
python3 tools/map_site.py

# 2-3. Build the Wayback index, then compute coverage + the missing list
python3 tools/archive_wayback.py index --delay 4
python3 tools/archive_wayback.py plan

# 4. Mirror everything Wayback has (resumable; safe to stop/restart)
python3 tools/archive_wayback.py fetch            # all ~11.2k pages
python3 tools/archive_wayback.py fetch --prefix /oxide/ --limit 3   # quick test
```

`archive_wayback.py all` runs index → plan → fetch in one shot.

## Politeness & ethics (non-negotiable)

- **The archiver never touches the live origin** — only `web.archive.org`.
  Wayback content outlives the June 26 shutdown, so there is no reason to add load.
- `map_site.py` is the *only* tool that contacts digitalfire.com, and it honors
  the site's `robots.txt` **`Crawl-delay: 20`** (we even hit a transient
  *Connection refused* during the survey — the server really is fragile).
- All tools are single-threaded, identify themselves via User-Agent + contact,
  cache to disk, and back off (respecting `Retry-After`) on 429/5xx.
- `/cgi-bin/`, `/videos/`, `/uploads/` are robots-disallowed and are never fetched live.

## Data outputs (`data/`)

| File | Tracked? | Description |
| --- | --- | --- |
| `urls.jsonl` / `urls.csv` | ✓ | Every sitemap URL + collection + lastmod |
| `survey_summary.{json,md}` | ✓ | Counts & path analysis per collection |
| `malformed_locs.json` | ✓ | 50 corrupt `<loc>` entries (digitalfire's video sitemap bug) |
| `wayback/cdx_index.jsonl` | ✓ | Latest HTTP-200 capture per archived URL (41,695) |
| `archive_plan.jsonl` | ✓ | URLs fetchable from Wayback + best timestamp |
| `missing_from_wayback.json` | ✓ | **The at-risk list (90)** — review before June 26 |
| `coverage_report.md` | ✓ | Per-collection Wayback coverage table |
| `wayback/cdx/` | ✗ | Raw paginated CDX cache (regenerable) |
| `archive/` | ✗ | Downloaded page snapshots (large; regenerable) |

## Data-quality findings

- **Sitemap host bug:** 11,340 of the sitemap's `<loc>` entries use
  `https://digitlfire.com` (missing the "a") — a host that doesn't resolve in DNS.
  The real domain is `digitalfire.com`; the tools canonicalize automatically.
- **Corrupt video sitemap:** 50/51 `<loc>`s in `sitemap-video.xml` contain a bare
  timestamp instead of a URL. Real video pages must be recovered from the CDX index.

## Roadmap (modernization)

1. ✅ Survey + map the site
2. ✅ Wayback coverage analysis + at-risk list + validated archiver
3. 🔄 Bulk-fetch all ~11.2k snapshots from Wayback (running)
4. ✅ Extract clean content (HTML → IR) preserving the cross-link graph — `prototype/crates/extract`
5. 🔄 Structured datasets — typed oxide chemistry wired into the prototype (materials/oxides + computed glaze UMF on recipes, with a Stull chart)
6. 🔄 Modern searchable site — **working prototype in [`prototype/`](prototype/)** (axum + Leptos + tantivy); offline ZIM/Kiwix + SQLite export still to do
7. ⬜ Publish to GitHub with provenance on every record

See **[`STATUS.md`](STATUS.md)** for the current handoff and **[`docs/research-report.md`](docs/research-report.md)** for the full research.

## Prototype

A modern, fast, searchable rebuild lives in [`prototype/`](prototype/) — it
converts the archived pages to an IR and serves them via **axum + Leptos (SSR +
hydration)** with **tantivy** full-text search.

```bash
cd prototype && just extract && just dev   # → http://127.0.0.1:8788
```

## Rights note

The owner has stated the shutdown stems from Terms-and-Conditions limits on the
Insight-Live account the source material was built from, and the Insight-Live
terms restrict redistribution of user content. Republishing carries real
copyright risk, and "open source" requires holding or being granted the rights.
This repo's tooling and metadata are safe to publish now; the *content* archive
should ideally be cleared with Tony Hansen before public redistribution. See the
research report in `docs/` for the full analysis. (Project direction: proceed with
the full open mirror — this note records the risk, not a blocker.)
