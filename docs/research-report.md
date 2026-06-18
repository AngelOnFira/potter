# Preserving Digitalfire.com: Research Synthesis & Action Plan

*A rights-respecting strategy to preserve the Digitalfire Ceramic Reference Library before its June 26, 2026 shutdown.*

---

## 1. Executive Summary

1. **The shutdown is a rights problem, not a hosting problem.** Tony Hansen states he "no longer [has] the authority to grant exemption to a section in the Terms and Conditions of using material in the Insight-Live account from which I built the source material" ([digitalfire.com/home](https://digitalfire.com/home)). This is a take-down obligation tied to mixed-ownership content. It is the single most important and most uncertain factor in the entire project.

2. **The legal/ethical bottom line: you cannot open-source what you do not own.** The corpus is a mix of Tony's own authorship, third-party/user-contributed content under the Insight-Live T&C, and a compilation. Even Tony likely cannot CC-license the *whole* site. A public verbatim mirror under no license, against an explicit anti-scrape clause and an active take-down, converts passive archival (Internet Archive) into active redistribution by an identifiable party — and would likely draw a DMCA/cease-and-desist.

3. **Recommended overall strategy — three layers, sequenced by rights clearance:**
   - **Layer A (do now, zero rights risk):** Preserve via the Internet Archive (which already holds ~34k–43k pages) and an immutable WARC/WACZ master. Publish only an **index/metadata/links catalog** that points to Wayback.
   - **Layer B (do now, low risk):** Extract the **uncopyrightable factual data** (oxide analyses, recipe ingredient amounts, melting points) and re-arrange it in an *original* structure for a tool/database. Facts are uncopyrightable in the US (Feist) and there is no US sui generis database right.
   - **Layer C (gated behind written permission):** A full-content public mirror (prose, images, page layout) — **only** after Tony licenses the parts he owns and the third-party content is cleared or excluded.

4. **A near-complete snapshot already exists — adopt it, don't re-crawl.** Akil Harris uploaded a **1.7 GB full-site archive to archive.org TODAY (2026-06-17)** — [archive.org/details/digitalfire-archive](https://archive.org/details/digitalfire-archive), 22,179 files, torrent-distributable (btih `c528fa3387bc283c6b7e964f534ee54a48c594dc`). This is the canonical raw download path and directly honors Tony's "stop downloading, IA has everything" request.

5. **A structured-dataset effort is already well advanced — coordinate, don't duplicate.** [github.com/millandr121/digitalfire](https://github.com/millandr121/digitalfire) already has ~3,100 cleaned JSON records (2,237 materials, 375 oxides, 216 minerals, 218 recipes, 72 temperatures) plus a Wayback-first CDX audit/fill pipeline and a glaze-calculator UI. Fork and contribute upstream rather than spinning up a parallel scraper.

6. **Wayback is the primary retrieval source — never touch the live origin.** CDX enumerates ~33,928 unique 200-status HTML pages (and ~43,205 unique URLs domain-wide across 6 subdomains, back to 1996). Raw, un-rewritten bytes are fetchable via the `id_` modifier, which hits *only* archive.org. Treat the live origin as off-limits.

7. **Throttling discipline is non-negotiable.** CDX tolerates ~60 requests/minute (1/sec). Ignoring HTTP 429 for over a minute triggers a **one-hour firewall-level IP ban that doubles on repeat**. Keep CDX strictly single-threaded with exponential backoff.

8. **The recommended modernization stack:** WARC/WACZ master → trafilatura extraction → Markdown+YAML-frontmatter per entity in Git + SQLite FTS5 → **Hugo + Pagefind** static site, a **ZIM for Kiwix** (offline-first for studios/schools with poor internet), and a **sql.js-httpvfs SQLite dump** for power-user SQL queries.

> **Contradiction flagged (site scale):** The dimensions report different totals because they measure different things. `content_structure` cites ~4,000–5,000 *Reference Database* pages (the curated, high-value corpus); `wayback_mechanics` measured **33,928 unique 200-status HTML pages**; `existing_efforts` measured **~43,205 unique URLs domain-wide** (6 subdomains, including redirects/assets/`4sight`/`cgi-local`). These are consistent once you distinguish "curated reference pages" < "unique HTML pages" < "all unique URLs including assets and redirects." Use ~4–5k as the *value* target and ~34k/43k as the *completeness* target.

> **Contradiction flagged (GoFundMe):** `rights_legal` documents a live GoFundMe ("Help save digitalfire!", ~$10,711 of $16,000, 145 donors — [gofundme.com/f/help-save-digitalfire](https://www.gofundme.com/f/help-save-digitalfire)); `existing_efforts` reports it could not find one. Treat the GoFundMe as **confirmed to exist** (the rights dimension verified it directly), and note it funds Tony's *own migration*, not open-sourcing.

---

## 2. Site Scale & Content Map

**What it is.** Digitalfire is functionally a **densely interlinked wiki** for ceramics/glaze chemistry authored over 35+ years by Tony Hansen (Plainsman Clays). Every page carries a *typed* "Related Information/Links" cross-reference block; ~15,000–27,000 interlinks bind the corpus into a graph. Material and recipe pages are heavily table-structured (oxide analyses, UMF/Seger formulas, batch line-items) and convert cleanly to JSON.

**Where the value is.** Per `sitemapindex.xml` (lastmod 2025-03-01) there are ~22 typed collections. Ranked by preservation priority:

| Tier | Collections | Approx. count | Why |
|---|---|---|---|
| **Highest** | `material` | ~2,834 | Chemistry tables (weight-% analysis + unity formula) — the core dataset |
| | `glossary` | ~349 | Definition essays (Eutectic, Stull Chart, UMF, Thixotropy) — irreplaceable prose |
| | `article` | ~110 | Long-form essays (Glaze Chemistry Basics, food-safety/leachability) |
| | `recipe` | ~79 | Batch ingredient lists + variations |
| | `trouble` | ~28 | Diagnostic Q&A (e.g. Glaze Crawling, ~1,500 words + photos) |
| | `oxide`/`mineral`/`hazard` | 109/109/92 | Chemistry & safety reference |
| **Second** | `property`, `schedule`, `temperature`, `test`, `typecode`, `project` | 14/29/73/100/~/~ | Cross-reference-graph integrity |
| | `potterytony` (Buy-A-Mug) | ~340 | Unique worked-examples with full provenance (serial, run, firing schedule, glaze, clay body) |
| **Lowest** | `consultants`, `schools`, `stores`, `url`, `misc` | — | Thin/stale directories; can skip |

**URL patterns:** `/oxide/NUM`, `/material/SLUG` (e.g. `/material/%23149+clay`), `/recipe/NUM` (e.g. `/recipe/g1214m`), `/glossary/term`, `/article/NUM`, `/test/`, `/typecode/NUM`.

**Caveats that shape the crawl:**
- Several collections (`consultants`, `schools`, `stores`, `url`, `typecode`, `project`, `potterytony`) have **empty dedicated sitemaps** despite having many live pages — so you **must** drive enumeration from CDX, not sitemaps alone, and cross-check against each page's inline A-Z collection index.
- `picture` (~2,693 caption pages / ~5,564 picture URLs) and `video` (~65–150) embed diagnostic photos (test tiles, micrographs, Stull charts) with real value — **capture binaries separately and rewrite inline image refs.**
- Domain-wide, the largest URL sections are non-reference: `4sight` (~10k), `cgi-local` (~4.4k), `url` redirect shortener (~3.6k). These inflate the 43k total but carry little unique value.

---

## 3. Wayback Machine Mechanics — Copy-Pasteable Playbook

**Golden rule:** all of the below hits **archive.org only** and never the live `digitalfire.com` origin. Keep CDX **single-threaded at ≤1 req/sec** with exponential backoff. Ignoring 429 for >60s = 1-hour firewall ban (doubles on repeat). Send a descriptive `User-Agent` with a contact email.

### (a) Enumerate archived URLs via CDX

Endpoint: `https://web.archive.org/cdx/search/cdx`

Build the complete HTML manifest with `resumeKey` pagination (the only mechanism reliable for *completeness* — `page`/`pageSize` omit the most recent captures):

```bash
# First page: domain match, only 200s, only HTML, one row per URL, with resume key
curl -s 'https://web.archive.org/cdx/search/cdx?\
url=digitalfire.com&matchType=domain&\
filter=statuscode:200&filter=mimetype:text/html&\
collapse=urlkey&\
fl=original,timestamp,digest,mimetype,statuscode&\
output=json&showResumeKey=true&limit=20000' \
-A 'digitalfire-preservation (you@example.com)'
# -> rows... then a blank line then a base64 resumeKey on its own line
```

Continue by re-issuing the *same* query with `&resumeKey=<TOKEN>` appended, looping until no key is returned. **Sleep ≥1s between calls.** Expect ~34k unique HTML rows.

Separate manifests for assets (drop or change the mimetype filter):
```bash
# Images:   &filter=mimetype:image/.*
# CSS:      &filter=mimetype:text/css
# JS:       &filter=mimetype:application/(x-)?javascript
```

Per-collection enumeration (validates sitemap-less collections):
```bash
curl -s 'https://web.archive.org/cdx/search/cdx?\
url=digitalfire.com/consultants*&matchType=prefix&\
collapse=urlkey&filter=statuscode:200&output=json' -A '...'
```

Useful CDX params: `matchType` = exact/prefix/host/domain · `collapse=urlkey` (one row/URL), `collapse=digest` (drop adjacent identical content), `collapse=timestamp:6` (monthly) · `from`/`to` (yyyyMMddhhmmss, 1–14 digits) · `limit=-N` (last N) · `filter=!statuscode:3..` (negate redirects). The `digest` column is a SHA-1 of the payload — use it to skip re-downloading identical bytes.

### (b) Fetch the best capture via `id_` (raw, un-rewritten bytes)

Format: `https://web.archive.org/web/<TIMESTAMP>id_/<ORIGINAL_URL>`

The 14-digit CDX `timestamp` plugs directly in. The `id_` modifier returns clean original bytes (verified: clean `<!DOCTYPE html>`, no toolbar, no link rewriting); the plain form injects a toolbar and rewrites links/JS. **Always use `id_` for preservation.** (`im_`/`if_` are raw/framed image variants.)

```bash
curl -s 'https://web.archive.org/web/20250313125330id_/https://digitalfire.com/material/%23149+clay' -A '...'
```

"Best capture" selection: CDX is sorted by `urlkey` then `timestamp`, and `collapse=urlkey` keeps the *first* (oldest) per group. To get the **latest** per URL, either fetch all rows per URL and sort by timestamp descending yourself, or use the Availability API for single URLs:

```bash
# Closest snapshot to a date for ONE url:
curl -s 'http://archive.org/wayback/available?url=digitalfire.com&timestamp=20250301'
# -> returns closest snapshot {available, url, timestamp, status}; build the id_ URL from it
```

Standardized enumeration via Memento (optional): TimeMap `https://web.archive.org/web/timemap/link/<URL>`; TimeGate `https://web.archive.org/web/<URL>` with an `Accept-Datetime` header → 302 to nearest memento.

### (c) Submit missing pages via Save Page Now (gap-fill ONLY)

Use **only** for pages the manifest shows are missing, and prioritize high-value pages before June 26, 2026. **Authenticate** — an authenticated account is essential for any sizeable job (12 concurrent sessions / 100,000 daily captures vs 6 / 4,000 anonymous).

```bash
# Get S3 keys from https://archive.org/account/s3.php, then:
curl -s -X POST 'https://web.archive.org/save' \
  -H "Authorization: LOW <accesskey>:<secret>" \
  -H "Accept: application/json" \
  --data-urlencode 'url=https://digitalfire.com/SOME/MISSING/PAGE' \
  --data 'if_not_archived_within=30d' \
  --data 'skip_first_archive=1'
# -> {"url":..., "job_id":...}
```

Poll the job:
```bash
curl -s 'https://web.archive.org/save/status/<job_id>' \
  -H "Authorization: LOW <accesskey>:<secret>"
# status: pending | success (timestamp, resources, outlinks) | error (status_ext code)
```

Key SPN2 params: `capture_all=1` (save 4xx/5xx), `capture_outlinks=1` (≤100/req), `if_not_archived_within=30d` (no-op fresh pages — **use this**), `js_behavior_timeout=N` (≤30s). Common error codes: `too-many-requests`, `blocked`, `soft-time-limit-exceeded` (45s), `too-many-daily-captures`, `filesize-limit` (2GB). Gate concurrency with `GET /save/status/user`. On `too-many-requests`, back off (subsequent captures on that host get 10–20s delays for ~60s).

> **Rate discrepancy (flagged):** SPN2 docs cite 6 concurrent / 4,000 daily anonymous; the `savepagenow` library cites ~3 captures/min anonymous, ~6/min authenticated. These are *different metrics* — concurrent-session and daily caps vs sustainable per-minute throughput. Pace by `/save/status/user` and stay well under daily caps; treat ~6/min authenticated as the practical sustained rate.

**Backoff template (CDX & retrieval):** on any 429, pause **all** workers and sleep 2→4→8→16s (cap ~60s); never retry immediately. Retrieval from `id_` can run 1–2 req/sec with jitter (it only loads the archive). Dedupe by CDX `digest`.

---

## 4. Preservation & Modernization Pipeline

End-to-end toolchain: **capture → extract → structure → publish (static site + ZIM + DB export)**, with provenance baked into every record.

### Capture (master)
- **Base layer:** download [archive.org/details/digitalfire-archive](https://archive.org/details/digitalfire-archive) (1.7 GB, 22,179 files) via **torrent** (btih `c528fa3387bc283c6b7e964f534ee54a48c594dc`) or archive.org webseeds; verify md5 `a2fb0e7920eed972db414b0cb0c931f1`. *Caveat:* 22,179 files vs ~43,205 Wayback URLs suggests this may be partial (possibly HTML-only) — diff against the CDX manifest.
- **Gap-fill & master format:** pull missing pages from Wayback via **waybackpack** (`pip install waybackpack`; flags `--raw` = `id_` form, `--uniques-only`, `--collapse timestamp:6`, **`--delay 1`**, `--user-agent` with email) or the **EDGI `wayback` library** (`pip install wayback`; `WaybackClient.search(...)` → `get_memento(..., mode=Mode.original)`; built-in ~1 search/sec + ~30 memento/sec rate limiting, retries, redirects). Reserve `waybackpy` for ad-hoc lookups.
- **Immutable provenance master:** keep one **WARC (ISO 28500)** / **WACZ** (WARC + CDXJ index + datapackage, browser-replayable, lossless superset). Pages are largely static, so `wget --warc` over Wayback `id_` snapshots suffices; reserve **Browsertrix Crawler** (Chromium-in-Docker) for any JS-rendered pages. Each WARC record carries `WARC-Target-URI` and `WARC-Date`. **Never re-run extraction against the live origin** — re-run it against the WARC.

### Extract
- **Primary:** **trafilatura** (`output_format='markdown'`, `include_tables=True`, `include_links=True`, `include_images=True`) — single-digit ms/page, automatic fallback to readability-lxml then jusText.
- **Domain tables:** supplement with **BeautifulSoup/lxml + pandoc** for the typed structures: oxide analyses (weight-% + unity formula), recipe batch line-items, UMF/Seger formulas, firing schedules. These need per-template parsing rules.

### Structure (the single normalized intermediate)
- **One Markdown file + YAML front matter per entity**, in a Git repo, directory hierarchy mirroring navigation (MDN `mdn/content` model). Front matter fields: `title, slug, type, source_url, archived_url, archived_timestamp, license_status` plus Dublin Core provenance (`dc:source`, `dcterms:created`, `dcterms:provenance`, `dcterms:rights`, `dcterms:rightsHolder` = Hansen/Digitalfire). This makes the corpus diffable, PR-able, and provenance-auditable.
- **Convert by type:** JSON for `oxide`/`material`/`recipe`/`property`/`schedule`/`typecode`; Markdown (preserving inline cross-links so the wiki graph survives) for `glossary`/`article`/`trouble`/`hazard`. Strip the repeated nav + full-collection A-Z index boilerplate.
- Adopt **DevDocs's separation of concerns:** a per-page filter chain emitting *normalized partials + index JSON + offline-data JSON*, distinct from index-building. That single intermediate feeds the static site, the ZIM, and the DB export **without re-scraping**.
- Build a **SQLite FTS5** (or DuckDB FTS) index from the same intermediate.

### Publish (three outputs, content-identical)
1. **Static site: Hugo + Pagefind** on GitHub Pages. Hugo builds 10k pages in <1 min (JS generators like Astro/Starlight and Docusaurus degrade past ~1,000 pages). **Pagefind** runs post-build and chunks its index so the browser downloads only relevant chunks (~50KB vs ~600KB monolithic). Add Meilisearch/Typesense only if typo-tolerant faceted *server* search is needed.
2. **ZIM for Kiwix** (offline-first — ceramics studios/schools often have poor internet; one ZIM gives the whole library offline with full-text search). Three routes: `warc2zim` (from WARC, fuzzy URL rewriting), `zimwriterfs` (from a local HTML dir; now in `openzim/zim-tools`), or `zimit` (Browsertrix → warc2zim, automated). Build from the **same normalized HTML** so site and ZIM stay identical.
3. **Queryable DB dump: SQLite via sql.js-httpvfs.** Serves a multi-hundred-MB DB directly from GitHub Pages using HTTP Range requests — a key lookup on a 670 MB DB transfers ~1 KB, no server, no full download. (GitHub Pages gzip can break HEAD content-length → specify file length in config.) Enables Stack-Exchange-Data-Explorer-style power-user SQL.

> **Rights gate on outputs:** the *index/metadata/links* and the *re-arranged factual DB* can publish now (see §6). The full-text site/ZIM mirror is **gated** behind written permission.

---

## 5. Existing Efforts & How to Coordinate (Don't Duplicate)

Most of this emerged in the last 24–48 hours. **Plug in; do not start from scratch.**

| Effort | What it is | How to coordinate |
|---|---|---|
| **[archive.org/details/digitalfire-archive](https://archive.org/details/digitalfire-archive)** (Akil Harris, `akil.harris@gmail.com`, uploaded 2026-06-17) | 1.7 GB full-site ZIP, 22,179 files, torrent btih `c528fa3387bc283c6b7e964f534ee54a48c594dc` | **Adopt as canonical raw snapshot.** Download via torrent/webseeds. |
| **[github.com/millandr121/digitalfire](https://github.com/millandr121/digitalfire)** (branch `claude/clever-einstein-e4qvvf`) | ~3,100 cleaned JSON records (2,237 materials w/ oxide analyses, 375 oxides, 216 minerals, 218 recipes, 72 temps) + glaze-calculator (React/Vite) + Wayback CDX audit/fill pipeline (`digitalfire_wayback.py`, `.github/workflows/wayback-fill.yml`). Records provenance-tagged. | **Fork and contribute upstream.** Open an issue offering to help rather than building a parallel scraper. Best structured base. |
| Shared **Google Drive** folder (ID `11uHARZFhcvMAXjBjAc7BZ2xKd7EcqN0h`, public) | Upstream archived HTML for the millandr121 pipeline; exists so contributors don't re-hit origin | Request access / mirror; **re-upload to archive.org as a proper WARC** for a durable IA-hosted home independent of one person's Drive. |
| `akilspots/digitalfire`, `Ashassins/digitalfire` (GitHub) | Empty placeholder repos created today (akilspots ≈ Akil Harris) | **Watch/contact, don't duplicate.** |
| **[Glazy](https://help.glazy.org/about)** + [derekphilipau/glazy-data](https://github.com/derekphilipau/glazy-data) (CC BY-NC-SA 4.0) | Established open ceramics-recipe corpus (Laravel; seeded from Sankey/Arbuckle/Katz, *not* digitalfire) | **Natural long-term institutional home** for preserved recipes/materials. Coordinate with Derek Au on licensing alignment. Independent corpus, not a mirror. |
| **[OpenGlaze](https://github.com/KyaniteLabs/openglaze)** | Open glaze tool; credits Digitalfire's oxide/UMF methodology; digitalfire import only on roadmap | Downstream **consumer/validator** of a clean dataset, not a mirror. |
| **[Ceramic Arts Daily "DigitalFire — kaboom"](https://community.ceramicartsdaily.org/topic/43991-digitalfire-kaboom)** thread + [Tony's Facebook](https://www.facebook.com/tonyatdigitalfire/posts) | Main human discussion hub (returned 403 to automated fetch) | **A human should read it** to capture volunteer names and any Discord/mailing-list coordination channel. |

**Coordination principle from the scripts:** the millandr121 pipeline already encodes the ethics-correct order — Wayback + shared Drive first, live origin only for documented gaps, and it explicitly frames recipe/material *factual chemistry data* as non-copyrightable. Align with that framing.

---

## 6. Rights & Ethics

### Facts (high confidence)
- **Shutdown reason (verbatim):** "I no longer have the authority to grant exemption to a section in the Terms and Conditions of using material in the Insight-Live account from which I built the source material. While there are ways to comply with the take-down order, they are beyond my means…" ([digitalfire.com/home](https://digitalfire.com/home)). It is a **rights/licensing compliance problem**, not cost.
- **Insight-Live T&C** ([insight-live.com/w3c/termsandconditions.php](https://insight-live.com/w3c/termsandconditions.php)): operator does **not** own user-entered Content ("We do not own any data… that your employees and staff enter into your account"); users grant only a limited operational license; an **explicit anti-scrape clause** ("…to spam, phish, pharm, pretext, spider, crawl, or scrape"); IP **non-transfer** ("This Agreement does not transfer to you any intellectual property owned by Digitalfire Corporation or third-parties…"). "Digitalfire" is a **trademark** of Digitalfire Corporation. Two distinct rights-holders named: Digitalfire Corporation **and** "third-parties" → mixed ownership confirmed.
- **US copyright law:** raw ceramic facts/data are **uncopyrightable** (Feist — [copyright.gov/reports/db4.pdf](https://www.copyright.gov/reports/db4.pdf)); selection/arrangement (compilation) and prose/images **are** protected; **no US sui generis database right**. You **cannot license what you do not hold** ([Creative Commons FAQ](https://creativecommons.org/faq/)) — so a CC0/CC-BY on a republished mirror without a documented chain of grants would be a *false license*.
- **GoFundMe** ([gofundme.com/f/help-save-digitalfire](https://www.gofundme.com/f/help-save-digitalfire)): "Help save digitalfire!", Tony Hansen, goal $16,000, ~$10,711 raised, 145 donors — funds Tony's **own migration**, not open-sourcing. No license grant, no invitation to mirror.

### Inferences (label as such)
- *Medium:* much of the long-form prose is plausibly Tony's own copyright (he is the author of the Reference Library), which he *could* in principle license — but the notice's wording shows the corpus is entangled with content he does **not** solely control.
- *Medium/High:* a public verbatim mirror to GitHub would likely draw a DMCA/cease-and-desist, given Tony was himself forced into a take-down.
- *Low:* the lost "authority" most plausibly stems from contractual obligations to other Insight-Live account holders/contributors (no evidence of a corporate sale/acquisition was found). **Ask Tony.**

### Safe to publish **now** vs. needs clearance

| Safe now (low/zero risk) | Needs written clearance first |
|---|---|
| Index/catalog of page titles, URLs, material/recipe identifiers + Wayback snapshot links (the "card catalog") | Verbatim full-content mirror (pages, prose, photographs) |
| Metadata schema + provenance fields | Any CC0/CC-BY relicensing of the corpus |
| **Re-arranged factual data** (oxide analyses, recipe amounts, melting/expansion values) in an *original* structure, with attribution | Reproduction of digitalfire's page layout/selection/prose |
| Link-outs to the Internet Archive | Use of the "Digitalfire" name/logo in a way implying endorsement |

### How to engage the owner
Engage Tony **first**, before publishing any full text/images. Reachable via the GoFundMe organizer contact, Instagram/Threads `@tonyatdigitalfire`, Facebook, and likely email via tonyhansen.com. **Lead with support for HIS migration goal** (donate / offer technical help), not "we are mirroring your site." Ask three concrete questions:
1. Which portions does he personally own and could license?
2. Is there content he is contractually barred from releasing (the Insight-Live T&C material)?
3. Would he endorse a links/metadata index that drives traffic to his migrated home?

### Throttling discipline (ethics)
- **Never** crawl or bulk-fetch the live `digitalfire.com` origin — it's overloaded, the T&C forbid spider/crawl/scrape, and Tony asked people to stop.
- Use Wayback/CDX (`id_` form) for all content inspection; it hits archive.org only.
- CDX ≤1 req/sec single-threaded with backoff; never ignore a 429 >60s (1-hour doubling firewall ban).
- Submission (gap-fill only) crawls from the IA side via SPN with `if_not_archived_within=30d`.
- Optionally push to a second archive (archive.today via `archivenow`) for redundancy on JS-heavy/dynamic pages.
- Document a takedown/clearance policy: clear DMCA contact, a "preservation, removed on owner request" statement, and a per-item provenance/rights field (owner-unknown vs Tony vs third-party).

---

## 7. Risk Register

| Risk | Likelihood | Mitigation |
|---|---|---|
| DMCA / cease-and-desist over a public verbatim mirror | Medium–High | Don't publish full content without written permission; publish index/metadata/links + re-arranged factual data only; document takedown policy. |
| Falsely relicensing mixed-ownership content (void CC license) | High *if attempted* | You cannot license what you don't own; gate any open release behind a documented chain of grants from Tony + third-party clearance/exclusion. |
| 1-hour (doubling) firewall IP ban from CDX 429s | High *if naive parallel crawler* | Single-threaded CDX ≤1 req/sec; mandatory exponential backoff (2/4/8/16s); pause all workers on 429. |
| Hammering the overloaded live origin (against T&C & owner request) | Medium | Never touch the origin; use the IA 1.7 GB archive, Wayback `id_`, and the shared Drive. |
| Duplicating existing volunteer effort | High | Adopt archive.org item; fork/contribute to millandr121; coordinate via the Ceramic Arts Daily thread before extracting. |
| IA archive is partial (22,179 files vs ~43,205 URLs; HTML-only?) | Medium | Diff the IA ZIP manifest against the CDX manifest; gap-fill missing pages from Wayback `id_`; capture image/binary assets separately. |
| Image/video binaries (test tiles, micrographs, Stull charts) not archived before shutdown | Medium | Enumerate a separate image manifest via CDX now; SPN-submit missing high-value binaries before June 26, 2026. |
| Single-person dependencies (one Google Drive, one uploader) disappear | Medium | Mirror the Drive to archive.org as a WARC; keep an immutable WARC/WACZ master under version control. |
| Dynamic/query-string pages (`picturedesc.php`, per-material queries) under-archived | Medium | Audit these specifically in CDX; prioritize for SPN gap-fill before shutdown. |
| Misrepresenting a sensitive legal dispute | Low–Medium | Quote Tony's own words; separate facts from inferences in any public writeup; don't use the Digitalfire mark to imply endorsement. |
| JS-rendered data (e.g. Desktop Insight XML export) lost in static capture | Low–Medium | Use Browsertrix for the handful of JS-heavy pages; confirm material/recipe tables are server-rendered. |

---

## 8. Recommended Next Steps (Ordered)

1. **Engage Tony Hansen first.** Send the support-led message (donate/offer help to his migration) and the three rights questions in §6. Do not publish any full text/images until you hear back.
2. **Have a human read the [Ceramic Arts Daily "kaboom" thread](https://community.ceramicartsdaily.org/topic/43991-digitalfire-kaboom)** and Tony's Facebook to capture volunteer names and any coordination channel (the 403 blocked automated reads).
3. **Adopt the existing snapshot.** Download [archive.org/details/digitalfire-archive](https://archive.org/details/digitalfire-archive) via torrent (btih `c528fa3387bc283c6b7e964f534ee54a48c594dc`); verify md5 `a2fb0e7920eed972db414b0cb0c931f1`. This is your raw base layer and honors the "stop downloading" request.
4. **Build the complete CDX manifest** (§3a): single resumeKey pull, `matchType=domain`, `filter=statuscode:200`, `filter=mimetype:text/html`, `collapse=urlkey`, `fl=original,timestamp,digest`, ≤1 req/sec. Build separate image/CSS/JS manifests. Expect ~34k HTML rows.
5. **Diff the IA ZIP (22,179 files) against the CDX manifest** to find gaps. Fill gaps from Wayback `id_` only (waybackpack `--raw --delay 1` or EDGI `wayback` `Mode.original`). Capture image binaries separately.
6. **Coordinate with millandr121:** open an issue, fork [github.com/millandr121/digitalfire](https://github.com/millandr121/digitalfire), and build on its JSON + CDX audit/fill pipeline. Request access to / mirror the shared Drive (`11uHARZFhcvMAXjBjAc7BZ2xKd7EcqN0h`) and re-upload it to archive.org as a WARC.
7. **Stand up the immutable master + normalized intermediate:** produce a WARC/WACZ from the gap-filled corpus; run trafilatura (+ per-type table parsers) to emit one Markdown/JSON file per entity with full provenance (Dublin Core) front matter; build SQLite FTS5.
8. **Publish the two safe-now layers:** (a) an index/metadata/links **catalog** (Hugo + Pagefind) linking to Wayback; (b) a **re-arranged factual dataset** (oxide analyses, recipe amounts) in an original structure with attribution. Keep the full-text site/ZIM behind the rights gate.
9. **Gap-fill submit (authenticated SPN2)** any high-value pages/images found missing, with `if_not_archived_within=30d`, **before June 26, 2026.** Prioritize by §2 value ranking; stay well under daily caps; gate via `/save/status/user`.
10. **If/when Tony grants a license:** upgrade the gated full-content site + ZIM, attach real per-record licenses, and coordinate long-term hosting with Glazy ([derekphilipau/glazy-data](https://github.com/derekphilipau/glazy-data)) / OpenGlaze.

---

### Open questions to resolve (decisive for scope)
- What exactly does the Insight-Live T&C clause cover — embedded third-party material, all of it? (Determines safe-to-publish vs preserve-only.)
- Will Tony grant an explicit open license, and for which portions? (Decides private archive vs public open dataset.)
- Is the 1.7 GB IA ZIP a true full mirror (HTML+images+CSS) or HTML-only? (Manifest diff needed.)
- Are recipe/material tables consistent enough for typed-column extraction, or do they need per-template rules?
- Does embedded imagery carry separate copyright requiring independent tracking?
- Is there a private volunteer coordination channel (Discord/mailing list) not surfaced by automated search?
