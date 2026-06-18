# Extraction-Defect Report: clay-knowledge Prototype

## 1. Executive Summary

The `crates/extract` pipeline produces **good substantive-content fidelity but poor boilerplate hygiene**. Across the 60 reviewed pages, the *real* encyclopedic payload (titles, descriptions, oxide-analysis tables, firing schedules, recipe ingredient tables, Notes/Details prose) is almost always preserved correctly, and internal link rewriting to `/collection/slug` routes mostly works. Fidelity is dragged down almost entirely by **what the extractor fails to strip and a small set of systematic link-rewrite bugs** — not by content loss.

**Average fidelity across the 60 pages: ≈ 71/100** (range 55–88).

The 3–5 highest-impact defect patterns, ranked by how many pages they affect:

1. **Footer/donation/social boilerplate not stripped** (~30+ pages). The digitalfire site-wide footer — PayPal donate table, "By Tony Hansen / Follow me on" social table, "Got a Question? / Buy me a coffee" Ko-Fi block, ReferenceLibrary logo, Privacy Policy — survives into `body_html` on nearly every glossary/hazard/material/mineral/oxide/property/recipe/schedule/temperature/test page. This is the single biggest fidelity sink and is highly mechanical to fix (one repeating template fragment).
2. **Leftover collapse/picker toggle anchor** (~35+ pages, nearly universal). The extractor correctly drops the giant picker list inside `<div id="collapse1">` but **keeps the toggle button** ("All Glossary" / "All Minerals" / "All Hazards" / "All Properties" / "All Temperature Numbers" / "All Recipe Codes" / "All Firing Schedules" / "All Troubles"), leaving a dead `href="#collapse1"` anchor at the very top of the body. One root cause, one fix.
3. **Spurious leading-slash on absolute social/external URLs** (~20+ pages). The link rewriter selectively corrupts exactly Threads, Instagram, and X/Twitter URLs to `/https://...` while leaving Facebook/LinkedIn/Pinterest/Ko-Fi intact — a faulty rewrite rule that would also damage *genuine* external content links, not just boilerplate.
4. **Wrong site-home + collection-scoped rewrites** (~20+ pages). `../home.html` is rewritten to `/<collection>/home` (e.g. `/temperature/home`, `/hazard/home`, `/material/home`), inventing nonexistent routes. Related: numeric-ID material links (`/material/806`) left un-slugified inline while the table uses slugs (`/material/gerstley-borate`) — inconsistent routing on the same page.
5. **Tail truncation / malformed-tag cutoffs** (several pages, occasionally high-severity). Output ends mid-tag/mid-attribute (e.g. `glossary/0-8mm-thickness` cut inside `<a href="/picture/24`, `property/glaze-color` cut mid-`<td>` dropping an entire Links section), producing malformed DOM and genuine content loss.

Fixing #1 and #2 alone would materially lift most pages in the 60–72 band into the 80s.

---

## 2. Defect Histogram (count by issue type × severity)

| Issue type | High | Medium | Low | Total |
|---|---:|---:|---:|---:|
| broken_or_wrong_link | 10 | 27 | 30 | 67 |
| boilerplate_noise | 13 | 24 | 6 | 43 |
| leftover_breadcrumb_or_nav | 3 | 28 | 4 | 35 |
| truncated_content | 1 | 2 | 9 | 12 |
| lost_image | 1 | 1 | 7 | 9 |
| formatting_broken | 0 | 3 | 5 | 8 |
| dropped_section | 2 | 0 | 2 | 4 |
| other | 0 | 1 | 4 | 5 |
| missing_table | 0 | 0 | 1 | 1 |
| wrong_or_messy_title | 0 | 0 | 2 | 2 |

(High-severity items shown were the independently-verified set; refuted entries already removed from the input.)

**Severity totals:** High = 30, Medium = 86, Low = 70.

---

## 3. Per-Collection Fidelity

| Collection | Pages | Avg fidelity | Worst page (fidelity) |
|---|---:|---:|---|
| article | 5 | 85.6 | `/article/111` and `/article/247` (82) |
| glossary | 5 | 66.4 | `/glossary/mdt` (60) |
| hazard | 5 | 68.4 | `/hazard/108` (62) |
| material | 6 | 73.7 | `/material/1-q-rok` (62) |
| mineral | 5 | 64.4 | `allophane`/`alunite`/`boracite` (62) |
| oxide | 5 | 75.6 | `/oxide/o` (62) |
| property | 5 | 63.2 | `body-color`/`body-plasticity`/`crystal-glaze-variations`/`glaze-color` (62) |
| recipe | 5 | 75.2 | `/recipe/bory1` (68) |
| schedule | 5 | 76.8 | `/schedule/brts6` (68) |
| temperature | 5 | 63.8 | `/temperature/1` (55) |
| test | 5 | 64.2 | `/test/glhd` (58) |
| trouble | 5 | 82.8 | `/trouble/glaze-blisters` (68) |

**Lowest-fidelity collections:** property (63.2), temperature (63.8), test (64.2), mineral (64.4) — all dominated by retained footer boilerplate + the picker-toggle leftover + leading-slash social links on thin pages where boilerplate is a large fraction of total body. **Highest:** article (85.6) and trouble (82.8), which are prose-heavy and lack the donation footer in most samples.

---

## 4. Top 10 Worst Pages

| # | Path | Fidelity | Key issues |
|---|---|---:|---|
| 1 | `/temperature/1` | 55 | HIGH boilerplate (PayPal/social/Ko-Fi/footer); HIGH leading-slash social URLs; leftover `#collapse1` nav; wrong `/temperature/home`; inconsistent slug-vs-numeric internal links |
| 2 | `/test/glhd` | 58 | HIGH boilerplate (PayPal table + author/social table); leftover picker nav; `/https://` social links; href-less Ko-Fi anchor; empty `<h3>Videos</h3>`; wrong `/test/home` |
| 3 | `/glossary/mdt` | 60 | HIGH leftover `#collapse1` nav; HIGH "Key phrases linking here" linkmaker block; HIGH PayPal + author/social tables; `/https://` social links; href-less anchor; inconsistent LOI link target |
| 4 | `/test/diel` | 61 | HIGH boilerplate footer; HIGH `/https://` social links; dead `#collapse1`; wrong `/test/home`; empty Videos heading; href-less Ko-Fi |
| 5 | `/glossary/foot-ring` | 62 | HIGH leftover `#collapse1` "All Glossary"; HIGH PayPal table; HIGH author/social table; malformed `/https://` socials + href-less "Buy me a coffee" |
| 6 | `/hazard/108` (Fluorine Gas) | 62 | HIGH `/https://` social links; leftover `#collapse1`; wrong `/hazard/home`; `/url/394`,`/url/383` redirect stubs likely dead; PayPal/Ko-Fi/footer boilerplate; stray empty `<tr>` |
| 7 | `/mineral/allophane` | 62 | HIGH leftover `#collapse1` "All Minerals"; HIGH PayPal+social+Ko-Fi boilerplate; `/https://` socials; wrong `/mineral/home` |
| 8 | `/property/glaze-color` | 62 | HIGH truncation (body cut mid-`<td>`, table left unclosed); HIGH dropped second "Links" section; leftover `#collapse1`; broken Mechanisms table |
| 9 | `/property/body-plasticity` | 62 | HIGH PayPal table; HIGH author/social table; HIGH `/https://` social links; leftover `#collapse1`; wrong `/property/home`; lost footer logo |
| 10 | `/property/crystal-glaze-variations` | 62 | HIGH `/https://` social links; HIGH full footer boilerplate; leftover `#collapse1`; wrong `/property/home`; href-less Ko-Fi |

(Several other pages also sit at 62: `oxide/o`, `mineral/alunite`, `mineral/boracite`, `material/1-q-rok`, `property/body-color`, `temperature/29`, `temperature/60`, `test/loi` — all driven by the same boilerplate + nav + leading-slash trio.)

---

## 5. Prioritized Extractor Fixes

### Fix 1 — Strip the digitalfire site-wide footer template
**What to change:** In the content-cleaning stage of `crates/extract`, add a boilerplate filter that removes the trailing footer cluster before serializing `body_html`. Detect and drop, as a unit: (a) any `<table>` containing `PayPalDonate.png` or the literal "No tracking, No ads, No paywall"; (b) any `<table>`/block containing "By Tony Hansen" / "Follow me on" (the social-icon row + `SignaturePhotoSmall.jpg`); (c) the `<h3>Got a Question?</h3>` + "Buy me a coffee and we can talk" + `ko-fi.svg` block; (d) the `ReferenceLibrary.svg` logo + "All Rights Reserved" + Privacy Policy footer. Anchor the rule on these stable signature strings/asset names so it generalizes across all collections.
**Resolves:** `boilerplate_noise` (43 occurrences, incl. all 13 high) and most of the `broken_or_wrong_link` low/medium items that live *inside* that footer (href-less "Buy me a coffee", `/w3c/privacypolicy.php`, wrong `/<collection>/home`). Also removes the `lost_image` ReferenceLibrary-logo noise as a non-issue.
**Effort:** Low–Medium. Single signature-based filter; the footer is byte-identical across pages.

### Fix 2 — Drop orphaned collapse/picker toggle anchors
**What to change:** When the picker `<div ... id="collapse1">` (or any `data-toggle="collapse"` target) is stripped, also strip the controlling toggle anchor. Concretely: in the same pass that drops the picker list, remove any `<a href="#collapse1">` (and more generally any anchor whose `href` is an in-page `#fragment` whose target element was removed, or any element carrying `data-toggle`/`role="button"`/`btn btn-primary` collapse semantics). Equivalent text labels to catch: "All Glossary / Minerals / Hazards / Properties / Temperature Numbers / Recipe Codes / Firing Schedules / Troubles" plus the sibling "All Recipes" / "Temperature Listing" / "All Tests" nav buttons.
**Resolves:** `leftover_breadcrumb_or_nav` (35 occurrences) and the paired `broken_or_wrong_link` dead-`#collapse1` items. Nearly universal across non-article pages.
**Effort:** Low. One rule keyed on the `#collapse1` fragment / collapse-toggle attributes.

### Fix 3 — Fix the leading-slash URL-rewrite bug for absolute URLs
**What to change:** In the link-rewriter, guard the path-normalization step so it **does not prepend `/` to hrefs that are already absolute** (start with `http://`/`https://`, or are protocol-relative). The current rule incorrectly mangles exactly the Threads/Instagram/X URLs (`/https://...`) while leaving Facebook/LinkedIn/Pinterest/Ko-Fi alone — indicating the bug triggers on a specific ordering/whitespace condition in those `<a>` tags. Add an early `if is_absolute_url(href) { leave unchanged }` branch before any slash-prefixing.
**Resolves:** `broken_or_wrong_link` (the `/https://` family — ~20 pages, including most of the high-severity link items). Critically, this rule would also corrupt *real* external content links, so it matters beyond boilerplate.
**Effort:** Low. A one-line guard, but verify against the inconsistency (write a test over the 7-link social block to confirm all 7 survive unchanged).

### Fix 4 — Correct site-home + slug normalization in the rewriter
**What to change:** (a) Map `../home.html` to a single canonical site-home route (e.g. `/home`) instead of resolving it relative to the current collection (`/temperature/home`, `/material/home`, etc.). (b) Make slug normalization consistent: convert *all* multi-word references' `+` separators to `-` (catch the missed `/glossary/transparent+glazes`, `/material/red+iron+oxide`, `/article/formulating+a+clear+glaze...`), and resolve inline numeric-ID links to the same slug form used in tables (the same material should not appear as both `/material/806` and `/material/gerstley-borate` on one page). Build the id→slug map once per crawl and apply it uniformly to inline prose, not just structured tables.
**Resolves:** `broken_or_wrong_link` (wrong `/<collection>/home`, ~12 pages; `+`-encoded slugs; numeric-vs-slug inconsistency across recipe/material/temperature/oxide pages).
**Effort:** Medium. The home-link and `+`→`-` parts are trivial; the inline-id→slug unification needs the shared id↔slug map and a prose-link pass.

### Fix 5 — Eliminate tail truncation and malformed-tag cutoffs
**What to change:** Investigate the body serializer's length/buffer handling — several outputs end mid-tag/mid-attribute (`<a href="/picture/24`, `<td><`, `<a href="/schedule`) with declared char counts (~15k) suggesting a hard cap or a streaming write that stops mid-token. Remove/raise any fixed truncation limit and ensure the HTML is closed/balanced before emit (run output through the same DOM serializer that guarantees closed tags). On `property/glaze-color` this truncation dropped a full Mechanisms-table tail *and* an entire "Links" section (HIGH `dropped_section`), so it is genuine content loss, not just a preview artifact.
**Resolves:** `truncated_content` (12, incl. the high `glossary/0-8mm-thickness` malformed cutoff) and the `dropped_section`/`formatting_broken` items caused by truncation on `property/glaze-color`. Also resolves the orphaned MP4 fallback (`lost_image` HIGH on `schedule/c10rpl`, also `article/247`) if combined with a `<video>`-preservation rule — emit the poster image + source link instead of only the "Your browser does not support MP4 video." fallback string.
**Effort:** Medium. Truncation/serializer fix is contained; `<video>` preservation is a small additional rule but distinct from the truncation work.

**Secondary / low-effort cleanups (bundle with the above):**
- Drop empty section headings (`<h3>Videos</h3>` with no content) and stray empty `<table></table>`, `<tr></tr>`, `<i></i>` placeholders (`formatting_broken`, ~8 pages).
- Optionally repair mojibake titles (`HÃ¼bnerite` → `Hübnerite`) and strip the "Key phrases linking here … /home/linkmaker" linkmaker block (glossary pages) — same signature-filter mechanism as Fix 1.
- Normalize the `description` subtitle styling loss (`text-muted font-italic`) only if visual fidelity is in scope — cosmetic, lowest priority.
