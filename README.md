# clay-knowledge

clay-knowledge is an open archive and modern rebuild of **[digitalfire.com](https://web.archive.org/web/2026/https://digitalfire.com)** — Tony Hansen's 35-year ceramics and glaze-chemistry reference, which shut down on 2026-06-26. It preserves the site from the Internet Archive and re-publishes the knowledge core — materials, oxides, minerals, glossary, articles, recipes, tests, hazards, and firing schedules — as a fast, searchable static site, enriched with typed oxide chemistry (analyses, Seger unity formulas, and Stull charts).

**Live:** [potter.forest-anderson.ca](https://potter.forest-anderson.ca)

---

- **[`prototype/`](prototype/)** — the static site + its build pipeline (axum/Leptos dev server, extractor, image optimizer, SSG, Pagefind). See [`prototype/README.md`](prototype/README.md) to build and deploy.
- **[`tools/`](tools/)** — throttled, Wayback-first survey + archiver scripts.
- **[`data/`](data/)** — the archived site, survey, and Wayback-coverage artifacts (the 1.8 GB source zip is on [archive.org](https://archive.org/details/digitalfire-archive)).
- **[`STATUS.md`](STATUS.md)** — current state and decisions · **[`docs/research-report.md`](docs/research-report.md)** — preservation research & rights notes.
