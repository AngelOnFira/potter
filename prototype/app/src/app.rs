use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::hooks::{use_params_map, use_query_map};
use leptos_router::path;
use serde::{Deserialize, Serialize};

use clay_ir::{CollectionInfo, Page};

/// Knowledge-core collections shown in the sidebar (slug, display title).
pub const COLLECTIONS: &[(&str, &str)] = &[
    ("material", "Materials"),
    ("oxide", "Oxides"),
    ("glossary", "Glossary"),
    ("article", "Articles"),
    ("recipe", "Recipes"),
    ("mineral", "Minerals"),
    ("hazard", "Hazards"),
    ("trouble", "Troubleshooting"),
    ("test", "Tests"),
    ("property", "Properties"),
    ("temperature", "Temperatures"),
    ("schedule", "Firing Schedules"),
];

// ---- transfer DTOs (compiled for both server and client) --------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Card {
    pub title: String,
    pub path: String,
    pub collection: String,
    pub summary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchHit {
    pub title: String,
    pub path: String,
    pub collection: String,
    pub snippet: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HomePayload {
    pub collections: Vec<CollectionInfo>,
    pub total: usize,
    pub featured: Vec<Card>,
}

// ---- server functions -------------------------------------------------------

#[server]
pub async fn home_data() -> Result<HomePayload, ServerFnError> {
    let s = crate::state::get();
    Ok(HomePayload {
        collections: s.collections.clone(),
        total: s.total,
        featured: s.featured.clone(),
    })
}

#[server]
pub async fn list_collection(name: String) -> Result<Vec<Card>, ServerFnError> {
    let s = crate::state::get();
    Ok(s.by_collection.get(&name).cloned().unwrap_or_default())
}

#[server]
pub async fn get_page(path: String) -> Result<Option<Page>, ServerFnError> {
    let s = crate::state::get();
    // resolve dedup aliases (e.g. /article/101 -> the canonical slug page)
    let page = s
        .pages
        .get(&path)
        .or_else(|| s.aliases.get(&path).and_then(|c| s.pages.get(c)))
        .cloned();
    Ok(page)
}

#[server]
pub async fn search(q: String) -> Result<Vec<SearchHit>, ServerFnError> {
    let s = crate::state::get();
    Ok(s.search.query(&q, 25))
}

// ---- shell + app ------------------------------------------------------------

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <link rel="icon" href="/favicon.svg"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="leptos" href="/pkg/clay-app.css"/>
        <Title text="Potter"/>
        <Router>
            <div class="layout">
                <Sidebar/>
                <div class="main">
                    <Topbar/>
                    <main class="content">
                        <Routes fallback=|| view! { <NotFound/> }>
                            <Route path=path!("/") view=Home/>
                            <Route path=path!("/search") view=SearchPage/>
                            <Route path=path!("/:collection") view=CollectionPage/>
                            <Route path=path!("/:collection/:slug") view=PageView/>
                        </Routes>
                    </main>
                </div>
            </div>
        </Router>
    }
}

#[component]
fn Sidebar() -> impl IntoView {
    view! {
        <nav class="sidebar">
            <a class="logo" href="/">"Potter"</a>
            <ul>
                {COLLECTIONS.iter().map(|(name, title)| view! {
                    <li><a href=format!("/{name}")>{*title}</a></li>
                }).collect_view()}
            </ul>
        </nav>
    }
}

#[component]
fn Topbar() -> impl IntoView {
    let q = RwSignal::new(String::new());
    let results = Resource::new(
        move || q.get(),
        |query: String| async move {
            if query.trim().len() < 2 {
                return Ok::<Vec<SearchHit>, ServerFnError>(Vec::new());
            }
            search(query).await
        },
    );
    view! {
        <header class="topbar">
            <div class="searchbox">
                <input
                    class="search-input"
                    type="search"
                    autocomplete="off"
                    placeholder="Search materials, oxides, glossary, articles…"
                    prop:value=move || q.get()
                    on:input=move |ev| q.set(event_target_value(&ev))
                />
                <div class="search-panel" class:show=move || !q.get().trim().is_empty()>
                    <Suspense>
                        {move || results.get().map(|res| match res {
                            Ok(hits) if hits.is_empty() => {
                                view! { <div class="sr-empty">"No matches"</div> }.into_any()
                            }
                            Ok(hits) => view! {
                                <ul class="sr-list">
                                    {hits.into_iter().map(|h| view! {
                                        <li>
                                            <a href=h.path.clone()>
                                                <span class=format!("sr-tag tag-{}", h.collection)>{h.collection.clone()}</span>
                                                <span class="sr-title">{h.title}</span>
                                                <span class="sr-snip" inner_html=h.snippet></span>
                                            </a>
                                        </li>
                                    }).collect_view()}
                                </ul>
                            }.into_any(),
                            Err(e) => view! { <div class="sr-empty">"Error: "{e.to_string()}</div> }.into_any(),
                        })}
                    </Suspense>
                </div>
            </div>
        </header>
    }
}

#[component]
fn Home() -> impl IntoView {
    let data = Resource::new(|| (), |_| async { home_data().await });
    view! {
        <Title text="Potter"/>
        <div class="home">
            <h1>"A modern, searchable ceramics reference"</h1>
            <p class="lede">
                "A preservation prototype of the digitalfire glaze-chemistry library: \
                 materials, oxides, recipes, and glossary, rebuilt as fast static pages."
            </p>
            <Suspense fallback=move || view! { <p class="muted">"Loading…"</p> }>
                {move || data.get().map(|res| match res {
                    Ok(d) => {
                        let total = d.total;
                        let ncol = d.collections.len();
                        view! {
                            <p class="muted">{total}" pages · "{ncol}" collections"</p>
                            <div class="grid">
                                {d.collections.into_iter().map(|c| view! {
                                    <a class="tile" href=format!("/{}", c.name)>
                                        <span class="tile-title">{c.title}</span>
                                        <span class="tile-count">{c.count}" pages"</span>
                                    </a>
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }
                    Err(e) => view! { <p class="error">"Error: "{e.to_string()}</p> }.into_any(),
                })}
            </Suspense>
        </div>
    }
}

#[component]
fn CollectionPage() -> impl IntoView {
    let params = use_params_map();
    let name = move || params.with(|p| p.get("collection").unwrap_or_default().to_string());
    let cards = Resource::new(name, |n| async move { list_collection(n).await });
    view! {
        <div class="collection">
            <Suspense fallback=move || view! { <p class="muted">"Loading…"</p> }>
                {move || cards.get().map(|res| match res {
                    Ok(cards) if cards.is_empty() => {
                        view! { <NotFound/> }.into_any()
                    }
                    Ok(cards) => {
                        let title = title_for(&cards.first().map(|c| c.collection.clone()).unwrap_or_default());
                        let n = cards.len();
                        view! {
                            <h1>{title}</h1>
                            <p class="muted">{n}" pages"</p>
                            <ul class="cardlist">
                                {cards.into_iter().map(|c| view! {
                                    <li class="cardrow">
                                        <a href=c.path.clone()>
                                            <span class="cardrow-title">{c.title}</span>
                                            <span class="cardrow-sum">{c.summary}</span>
                                        </a>
                                    </li>
                                }).collect_view()}
                            </ul>
                        }.into_any()
                    }
                    Err(e) => view! { <p class="error">{e.to_string()}</p> }.into_any(),
                })}
            </Suspense>
        </div>
    }
}

#[component]
fn PageView() -> impl IntoView {
    let params = use_params_map();
    let route = move || {
        params.with(|p| {
            format!(
                "/{}/{}",
                p.get("collection").unwrap_or_default(),
                p.get("slug").unwrap_or_default()
            )
        })
    };
    let page = Resource::new(route, |r| async move { get_page(r).await });
    view! {
        <Suspense fallback=move || view! { <p class="muted">"Loading…"</p> }>
            {move || page.get().map(|res| match res {
                Ok(Some(pg)) => {
                    let coll = pg.collection.clone();
                    let coll_title = title_for(&coll);
                    let related = pg.related.clone();
                    let chem = pg.chemistry.clone();
                    let src = pg.source_url.clone();
                    let src2 = pg.source_url.clone();
                    view! {
                        <Title text=pg.title.clone()/>
                        <article class="article">
                            <nav class="crumbs">
                                <a href="/">"Home"</a>" / "
                                <a href=format!("/{coll}")>{coll_title}</a>
                            </nav>
                            {chem.map(|c| {
                                let rows = c.analysis;
                                let props = c.properties;
                                let formula = c.formula;
                                let (ow, fw) = (c.oxide_weight, c.formula_weight);
                                let has_rows = !rows.is_empty();
                                let has_props = !props.is_empty();
                                // UMF Stull point from SiO2 (x) and Al2O3 (y)
                                let sio2 = rows.iter().find(|o| o.oxide == "SiO2").and_then(|o| o.formula);
                                let al2o3 = rows.iter().find(|o| o.oxide == "Al2O3").and_then(|o| o.formula);
                                // Stull chart: plot area x 44..300 (SiO2 0-5), y 16..180 (Al2O3 0-1)
                                let stull = match (sio2, al2o3) {
                                    (Some(x), Some(y)) if x > 0.0 => {
                                        let cx = 44.0 + (x.clamp(0.0, 5.0) / 5.0) * 256.0;
                                        let cy = 180.0 - (y.clamp(0.0, 1.0) / 1.0) * 164.0;
                                        Some((
                                            format!("{cx:.1}"),
                                            format!("{cy:.1}"),
                                            format!("SiO\u{2082} {x:.2} \u{00b7} Al\u{2082}O\u{2083} {y:.2}"),
                                        ))
                                    }
                                    _ => None,
                                };
                                view! {
                                    <section class="chem">
                                        <h3>"Chemistry"</h3>
                                        {formula.map(|f| view! { <p class="chem-formula">{f}</p> })}
                                        {has_rows.then(|| view! {
                                            <table class="chem-table">
                                                <thead><tr>
                                                    <th>"Oxide"</th><th>"Analysis %"</th>
                                                    <th>"Unity (UMF)"</th><th>"\u{00b1}"</th>
                                                </tr></thead>
                                                <tbody>
                                                    {rows.into_iter().map(|o| view! {
                                                        <tr>
                                                            <td class="ox">{o.oxide}</td>
                                                            <td>{o.analysis_pct.map(|v| format!("{v:.2}")).unwrap_or_default()}</td>
                                                            <td>{o.formula.map(|v| format!("{v:.3}")).unwrap_or_default()}</td>
                                                            <td class="tol">{o.tolerance.unwrap_or_default()}</td>
                                                        </tr>
                                                    }).collect_view()}
                                                </tbody>
                                            </table>
                                        })}
                                        {stull.map(|(cx, cy, label)| view! {
                                            <figure class="stull-fig">
                                                <svg class="stull" width="320" height="210">
                                                    <rect x="44" y="16" width="256" height="164" fill="none" stroke="#d9d4cc"></rect>
                                                    // vertical gridlines (SiO2 1..5)
                                                    <line x1="95.2" y1="16" x2="95.2" y2="180" stroke="#efeae2"></line>
                                                    <line x1="146.4" y1="16" x2="146.4" y2="180" stroke="#efeae2"></line>
                                                    <line x1="197.6" y1="16" x2="197.6" y2="180" stroke="#efeae2"></line>
                                                    <line x1="248.8" y1="16" x2="248.8" y2="180" stroke="#efeae2"></line>
                                                    // horizontal gridlines (Al2O3 .2..1.0)
                                                    <line x1="44" y1="147.2" x2="300" y2="147.2" stroke="#efeae2"></line>
                                                    <line x1="44" y1="114.4" x2="300" y2="114.4" stroke="#efeae2"></line>
                                                    <line x1="44" y1="81.6" x2="300" y2="81.6" stroke="#efeae2"></line>
                                                    <line x1="44" y1="48.8" x2="300" y2="48.8" stroke="#efeae2"></line>
                                                    // x ticks
                                                    <text x="92" y="194" class="tick">"1"</text>
                                                    <text x="143" y="194" class="tick">"2"</text>
                                                    <text x="194" y="194" class="tick">"3"</text>
                                                    <text x="245" y="194" class="tick">"4"</text>
                                                    <text x="296" y="194" class="tick">"5"</text>
                                                    // y ticks
                                                    <text x="22" y="151" class="tick">"0.2"</text>
                                                    <text x="22" y="118" class="tick">"0.4"</text>
                                                    <text x="22" y="85" class="tick">"0.6"</text>
                                                    <text x="22" y="52" class="tick">"0.8"</text>
                                                    <text x="22" y="20" class="tick">"1.0"</text>
                                                    // axis titles
                                                    <text x="150" y="207" class="ax">"SiO\u{2082} (UMF)"</text>
                                                    <text x="14" y="104" class="ax" transform="rotate(-90 14 104)">"Al\u{2082}O\u{2083} (UMF)"</text>
                                                    // approximate Stull zones (cone ~10)
                                                    <text x="60" y="120" class="zone">"crazing"</text>
                                                    <text x="196" y="126" class="zone">"glossy"</text>
                                                    <text x="150" y="44" class="zone">"matte"</text>
                                                    <text x="236" y="40" class="zone">"under-fired"</text>
                                                    <text x="96" y="172" class="zone">"fluid"</text>
                                                    <circle cx=cx cy=cy r="5.5" fill="#b5502a" stroke="#fff" stroke-width="1.5"></circle>
                                                </svg>
                                                <figcaption class="stull-cap">
                                                    "Stull position: "{label}" (approximate zones, ≈cone 10)"
                                                </figcaption>
                                            </figure>
                                        })}
                                        {has_props.then(|| view! {
                                            <table class="chem-table props">
                                                <tbody>
                                                    {props.into_iter().map(|kv| view! {
                                                        <tr><td class="ox">{kv.key}</td><td class="pv">{kv.val}</td></tr>
                                                    }).collect_view()}
                                                </tbody>
                                            </table>
                                        })}
                                        <p class="chem-meta">
                                            {ow.map(|w| format!("Oxide weight {w:.2}"))}
                                            {fw.map(|w| format!(" \u{00b7} Formula weight {w:.2}"))}
                                        </p>
                                    </section>
                                }
                            })}
                            <div class="article-body" inner_html=pg.body_html></div>
                            {(!related.is_empty()).then(|| view! {
                                <aside class="related">
                                    <h3>"Related"</h3>
                                    <ul>
                                        {related.into_iter().map(|l| view! {
                                            <li><a href=l.href>{l.label}</a></li>
                                        }).collect_view()}
                                    </ul>
                                </aside>
                            })}
                            <footer class="provenance">
                                "Archived from "
                                <a href=src target="_blank" rel="noopener">{src2}</a>
                            </footer>
                        </article>
                    }.into_any()
                }
                Ok(None) => view! { <NotFound/> }.into_any(),
                Err(e) => view! { <p class="error">{e.to_string()}</p> }.into_any(),
            })}
        </Suspense>
    }
}

#[component]
fn SearchPage() -> impl IntoView {
    let query = use_query_map();
    let q = move || query.with(|m| m.get("q").unwrap_or_default().to_string());
    let results = Resource::new(q, |query: String| async move {
        if query.trim().is_empty() {
            return Ok::<Vec<SearchHit>, ServerFnError>(Vec::new());
        }
        search(query).await
    });
    view! {
        <div class="searchpage">
            <h1>"Search"</h1>
            <p class="muted">"Query: "{move || q()}</p>
            <Suspense fallback=move || view! { <p class="muted">"Searching…"</p> }>
                {move || results.get().map(|res| match res {
                    Ok(hits) if hits.is_empty() => view! { <p class="muted">"No results."</p> }.into_any(),
                    Ok(hits) => view! {
                        <ul class="cardlist">
                            {hits.into_iter().map(|h| view! {
                                <li class="cardrow">
                                    <a href=h.path.clone()>
                                        <span class=format!("sr-tag tag-{}", h.collection)>{h.collection.clone()}</span>
                                        <span class="cardrow-title">{h.title}</span>
                                        <span class="cardrow-sum" inner_html=h.snippet></span>
                                    </a>
                                </li>
                            }).collect_view()}
                        </ul>
                    }.into_any(),
                    Err(e) => view! { <p class="error">{e.to_string()}</p> }.into_any(),
                })}
            </Suspense>
        </div>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <div class="notfound">
            <h1>"Not found"</h1>
            <p class="muted">"That page isn't in this prototype's knowledge-core slice."</p>
            <a href="/">"Back home"</a>
        </div>
    }
}

fn title_for(collection: &str) -> String {
    COLLECTIONS
        .iter()
        .find(|(name, _)| *name == collection)
        .map(|(_, title)| title.to_string())
        .unwrap_or_else(|| collection.to_string())
}
