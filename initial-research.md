I found out about this ```Please, everyone, don't panic and stop downloading to save all the pages, the ISP is warning me about overload. The internet archive has everything. Read your social media to find out what is going on. I am taking this down for a few hours to set up throttling. Please be patient. I’ll post at https://www.instagram.com/tonyatdigitalfire and https://www.facebook.com/tony.hansen.3990/

Digitalfire will shut down on June 26. I no longer have the authority to grant exemption to a section in the Terms and Conditions of using material in the Insight-Live account from which I built the source material. While there are ways to comply with the take-down order, they are beyond my means because of how complex and large the site has become in the past 35+ years. It has been a wild ride for a shy prairie boy, thanks to everyone for your support. Insight-live.com general operation is not affected by this.``` on this site https://digitalfire.com/ and i want to consider how to offer the information in a more modern format. I'm considering trying to download the site from internet archive, converting it to a more modern format, then making a new site, and putting all of the content on github to be open source. I have a few questions. is it easy to download this much from the internet archive? can you look into prior art on how different kinds of data have been preserved, and made more usable? Can you look more into what it means with the takedown request? I think the wayback machine is good, but it's not super easy to access for people who want this information.

---

My take: **don’t start by republishing a full Digitalfire mirror**. Start by helping create a **rights-cleared preservation pipeline**. The technical archive work is manageable; the legal/permission layer is the hard part.

## What I found

Digitalfire’s own homepage confirms the notice you saw: people were hammering the site, the ISP warned about overload, Tony says “the internet archive has everything,” and Digitalfire is slated to shut down on **June 26** while Insight-Live itself is “not affected.” ([DigitalFire][1]) The site also says there is a GoFundMe and that he is receiving many offers of help from coder/potters. ([DigitalFire][1])

The important legal clue is in the Insight-Live terms. They say **user content remains owned by the account user**, and Digitalfire only gets permission to access/copy/store/display it as needed to provide the service; unless the user specifically permits it, the terms say the website does **not** get a license to use, reproduce, adapt, publish, or distribute that content for commercial/marketing/similar purposes. ([Insight Live][2]) The terms also prohibit using the website/content to “spider, crawl, or scrape,” and say IP rights are not transferred. ([Insight Live][2])

So the takedown probably means: **some Digitalfire public pages were built from material inside an Insight-Live account whose owner/authority situation no longer permits that use**. That is an inference, but it matches the shutdown text and the terms. The risky part for you is that copying the site from Wayback and putting it on GitHub could preserve the exact same rights problem, and making it “open source” would go even further by granting rights you may not have.

## Is it easy to download this much from Internet Archive?

**Technically possible, not “easy.”** The Internet Archive exposes Wayback data through APIs: the Availability API, the CDX API for querying captures, and Memento; the `wayback` Python client can search CDX and fetch raw archived content. ([Internet Archive][3]) ([Wayback][4]) But Internet Archive says it is **not a general backup service**, does not guarantee a site is fully archived, and no longer offers a service to package up lost sites. ([Internet Archive Help Center][5]) It also warns that missing images, JavaScript, robots.txt, server-side features, and incomplete captures can break archived pages. ([Internet Archive Help Center][5])

A typical workflow would be:

```bash
# 1. Query unique archived URLs through CDX
# 2. Filter to status 200, text/html, images, PDFs, etc.
# 3. For each capture, fetch the raw archived URL form:
#    https://web.archive.org/web/{timestamp}id_/{original_url}
# 4. Rewrite links, dedupe by digest, extract content, and preserve provenance.
```

You would need aggressive throttling and backoff. Community tooling around Wayback has reported CDX rate-limit/429 issues around bulk use, so a preservation crawler should behave politely rather than parallel-download. ([GitHub][6]) Also, Internet Archive can remove or exclude archived pages after review when a site/account owner requests it, so Wayback should not be treated as permanent rights clearance. ([Internet Archive Help Center][5])

## Prior art worth copying

The best examples split into two layers: **preservation format** and **usable access format**.

**For preservation**, use WARC/WACZ-style archival captures. Tools like ArchiveBox save rendered HTML, screenshots, PDFs, WARC files, article text, headers, metadata, and ordinary files/SQLite/JSON so the archive is not trapped in one proprietary app. ([ArchiveBox][7]) The IIPC web-archiving ecosystem lists tools like Browsertrix, ArchiveWeb.Page, Heritrix, grab-site, SingleFile, and WARC tools for higher-fidelity capture. ([GitHub][8])

**For modern documentation**, look at MDN: its docs are in an open GitHub repository and written in Markdown-ish formats with contribution rules, rather than being only a scraped website. ([GitHub][9]) ([MDN Web Docs][10]) DevDocs is another useful model: it converts many documentation sources into a clean, searchable, offline-capable interface. ([GitHub][11])

**For bulk knowledge dumps**, Wikipedia and Stack Exchange are the models. Wikimedia publishes structured dumps regularly, with rate limits and licensing information. ([Wikimedia Downloads][12]) Stack Exchange’s public archive.org data dump shows how a community knowledge base can be distributed as compressed datasets rather than only as browsable pages. ([Internet Archive][13])

**For offline access**, ZIM/Kiwix is a strong model. `zimwriterfs` can package a directory of self-contained HTML into a ZIM file readable by Kiwix. ([GitHub][14]) That might actually be perfect for ceramics studios, schools, and people with poor internet.

## What I’d build instead

The safest useful project would be:

1. **Digitalfire Archive Index** — a searchable index of Digitalfire URLs, titles, categories, and Wayback links, without republishing the full text/images yet.

2. **Rights-cleared content repo** — Markdown/JSON only for pages Tony or rights holders explicitly clear. GitHub can host the code and schema immediately, but not necessarily all content. GitHub notes that a public repo is not truly open source unless it is licensed, and without a license default copyright applies. ([GitHub Docs][15])

3. **Structured ceramics dataset** — separate facts from expression:

   * materials
   * oxides
   * recipes
   * firing schedules
   * glossary terms
   * image metadata
   * source URL
   * archived timestamp
   * license/permission status

4. **Modern front-end** — static site plus search, maybe Astro/Vite/Docusaurus/Pagefind or a small Rust/Next stack. The key is to preserve provenance on every page.

5. **Offline export** — generate a ZIM/Kiwix package and maybe a SQLite/DuckDB export.

The repo could look like:

```text
digitalfire-preservation/
  data/
    materials/
    oxides/
    recipes/
    glossary/
    schedules/
  archive-index/
    wayback-cdx.jsonl
    url-map.csv
  content/
    cleared/
    restricted/
  tools/
    import-wayback/
    extract-html/
    normalize-materials/
    build-zim/
  site/
  LICENSE-CODE
  CONTENT-LICENSE.md
  RIGHTS-STATUS.md
```

## What not to do

I would **not** scrape the live site right now. Tony’s message says the ISP warned him about overload. ([DigitalFire][1]) I also would **not** publish a GitHub repo containing all mirrored Digitalfire pages unless Tony and the relevant rights holders approve. Creative Commons explicitly says a publisher needs the necessary rights before applying a CC license, especially when a database contains material from other authors. ([Creative Commons][16]) ([Creative Commons][16])

The most helpful first move is probably to contact Tony with something like: “I can help build a rights-audited export/import pipeline and a modern static archive, but I won’t republish anything unless you can identify what is cleared.” That aligns with the problem he describes: there are ways to comply, but the site is too complex and large for him to do alone.

[1]: https://digitalfire.com/home "Digitalfire.com Home Page"
[2]: https://insight-live.com/w3c/termsandconditions.php "Accept Digitalfire Terms of Service"
[3]: https://archive.org/help/wayback_api.php "Wayback Machine APIs | Internet Archive"
[4]: https://wayback.readthedocs.io/en/v0.2.6/usage.html "Usage — wayback 0.2.6 documentation"
[5]: https://help.archive.org/help/using-the-wayback-machine/ "Using the Wayback Machine – Internet Archive Help Center"
[6]: https://github.com/edgi-govdata-archiving/wayback/issues/137 "search_calls_per_second needs to be dialed down · Issue #137 · edgi-govdata-archiving/wayback · GitHub"
[7]: https://archivebox.io/ "ArchiveBox - Open-source self-hosted web archiving"
[8]: https://github.com/iipc/awesome-web-archiving/blob/main/README.md "awesome-web-archiving/README.md at main · iipc/awesome-web-archiving · GitHub"
[9]: https://github.com/mdn/content "GitHub - mdn/content: The official source for MDN Web Docs content. Home to over 14,000 pages of documentation about HTML, CSS, JS, HTTP, Web APIs, and more. · GitHub"
[10]: https://developer.mozilla.org/en-US/docs/MDN/Writing_guidelines "Writing guidelines - MDN Web Docs | MDN"
[11]: https://github.com/freecodecamp/devdocs "GitHub - freeCodeCamp/devdocs: API Documentation Browser · GitHub"
[12]: https://dumps.wikimedia.org/backup-index.html "Wikimedia Downloads"
[13]: https://archive.org/download/stackexchange "stackexchange directory listing"
[14]: https://github.com/openzim/zim-tools "GitHub - openzim/zim-tools: Various ZIM command line tools · GitHub"
[15]: https://docs.github.com/articles/licensing-a-repository "Licensing a repository - GitHub Docs"
[16]: https://creativecommons.org/faq/ "Frequently Asked Questions - Creative Commons"
