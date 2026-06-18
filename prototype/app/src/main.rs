#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use clay_app::app::*;
    use leptos::logging::log;
    use leptos::prelude::*;
    use axum::routing::get_service;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use tower_http::services::{ServeDir, ServeFile};

    // Load the IR + build the tantivy search index before serving.
    clay_app::state::init();

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let site_root = leptos_options.site_root.to_string();
    let routes = generate_route_list(App);

    // Serve the archived images/videos from the extracted ZIP at /media/*.
    let media_dir = std::env::var("CLAY_MEDIA").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/digitalfire-archive/media").to_string()
    });

    // Static assets are served explicitly because the app uses a greedy
    // `/:collection/:slug` route that would otherwise capture `/pkg/...` etc.
    let app = Router::new()
        .nest_service("/pkg", ServeDir::new(format!("{site_root}/pkg")))
        .nest_service("/media", ServeDir::new(media_dir))
        .route("/favicon.svg", get_service(ServeFile::new(format!("{site_root}/favicon.svg"))))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // hydration entry lives in lib.rs
}
