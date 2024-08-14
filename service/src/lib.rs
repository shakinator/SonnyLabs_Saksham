mod web;
pub mod db;
mod ml;
mod python;
mod v1;
mod auth;
mod header;

use axum::{
    extract::FromRef,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};

use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

type MJEnvironment = minijinja::Environment<'static>;

#[derive(FromRef, Clone)]
pub struct AppState {
    pub mj_environment: MJEnvironment,
    pub db_pool: sqlx::PgPool,
}

pub type AppRouter = Router<AppState>;

pub struct AppError(anyhow::Error);
type AppResult<T> = Result<T, AppError>;

pub async fn serve() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or("debug,axum::rejection=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();


    let enable_ml: bool = std::env::var_os("SL_ENABLE_ML")
        .unwrap_or("1".into())
        == "1";

    if enable_ml {
        // Setup python, python modules, and python actor thread
        python::init();

        // Boot any thread-sensitive python here
        ml::prompt_injection::init();
        ml::toxicity::init();
        ml::pii::init();

        python::init_python_thread();
    }

    // Allow for python compile in Dockerfile
    if std::env::var_os("SL_PYTHON_COMPILE_ONLY").unwrap_or("0".into()) == "1" {
        return
    }


    // load templates
    let mut mj_environment = MJEnvironment::new();
    mj_environment
        .add_template("base.html", BASE_TEMPLATE)
        .unwrap();

    web::analysis::add_templates(&mut mj_environment);
    web::auth::add_templates(&mut mj_environment);
    web::admin::add_templates(&mut mj_environment);


    // connect to db
    let db_pool = db::connect().await;


    // prepare state
    let app_state = AppState {
        mj_environment,
        db_pool,
    };


    // build our application with a route
    let web_app = AppRouter::new()
        // authed
        .nest("/", web::admin::router(app_state.clone()))
        .nest("/", web::analysis::router())
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(), auth::web::middleware
        ))
        // not authed
        .route("/", get(|| async { Redirect::permanent("/analysis/") }))
        .nest("/", web::auth::router())
        .with_state(app_state.clone());

    let rest_app = AppRouter::new()
        // authed
        .nest("/", v1::analysis::router())
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(), auth::api::middleware
        ))
        .with_state(app_state.clone());

    let app = Router::new()
        .nest("/", web_app)
        .nest("/", rest_app)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );


    // Setup multiprocess-friendly binding
    let socket = socket2::Socket::new(
        socket2::Domain::IPV4, socket2::Type::STREAM, Some(socket2::Protocol::TCP)
    ).unwrap();
    socket.set_nonblocking(true).unwrap();
    socket.set_reuse_port(true).unwrap(); // Allow multiple (linux) processes to bind
    let addr: std::net::SocketAddr = "0.0.0.0:3000".parse().unwrap();
    socket.bind(&addr.into()).unwrap();
    socket.listen(1024).unwrap(); // mio listener default

    let listener = tokio::net::TcpListener::from_std(socket.into()).unwrap();

    // run it
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

const BASE_TEMPLATE: &'static str = r#"
<!doctype html>
<html lang="en" class=-no-dark-theme >
<head>
    <title>{% block title %}SonnyAI{% endblock title %}</title>
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    <link rel="stylesheet" href="https://unpkg.com/missing.css@1.1.1">
    <script type="module" src="https://unpkg.com/missing.css@1.1.1/dist/js/tabs.js"></script>
    <style type="text/css">
        :root {
            --gap: 1rem;

            /*
            --accent: hsl(228, 78%, 55%);
            --accent-secondary: hsl(108, 78%, 55%);
            */
            --palatte-1: hsl(212, 33%, 32%);
            --palatte-2: hsl(183, 56%, 52%);
            --palatte-3: hsl(0, 0%, 96%, 1);
            --palatte-4: hsl(342, 97%, 65%, 1);

            --bg: var(--gray-0);
            --bg-light: var(--gray-0);
            --bg-dark: var(--gray-12);

            --fg: var(--gray-12);
            --fg-dark: var(--gray-12);
            --fg-light: var(--gray-0);

            --accent: var(--palatte-1);
            --accent-fg: var(--fg-light);
            --bg-zebra: hsl(212, 33%, 95%);

            --accent-secondary: var(--palatte-2);
            --accent-secondary-fg: var(--fg-dark);

            /* table zebra bg, v.light accent */
            --bg-zebra: hsl(228, 78%, 97%);

            --color-primary: #3054E6;
            --color-secondary: #54E630;
            --color-red: #E63054;
            --color-black: #222222;
            --color-text-black: #222222;
            --color-text-white: #EEEEEE;
            --color-bg-black: #222222;
            --color-bg-white: #EEEEEE;
            --color-bg-white-darker: #DDDDDD;

            // background: var(--color-bg-white);
            // color: var(--color-text-black);
        }
        .container {
            max-width: 1000px;
            margin: 0 auto;
        }
        body > header {
            padding: 10px 0;
            box-shadow: 0 0 2px;
            background: var(--accent);
            color: var(--accent-fg);
        }
        body > header * {
            color: var(--accent-fg);
        }
        .logo {
            text-decoration: none;
            font-size: 24px;
            padding-right: 20px;
            font-family: sans-serif;
        }
        .logo:hover {
            text-decoration: none;
        }


        /* .table-container */
        /* Padding */
        .table-container > table > thead > tr > th {
            //padding: 20px;
            padding: var(--gap)
        }
        .table-container > table > tbody > tr > th {
            padding: 10px 20px;
        }
        .table-container > table > tbody > tr > td {
            padding: 10px 20px;
        }
        .table-container > table > tbody > tr:nth-child(odd) {
            background-color: var(--bg-zebra);
        }
        /* Rounded outer border */
        .table-container {
            width: 100%;
            border: 1px solid var(--graphical-fg);
            border-radius: 5px;
        }
        /* Add internal (td) borders */
        .table-container > table > tbody > tr > td {
            border: 1px solid var(--graphical-fg);
        }
        /* remove outer td borders */
        .table-container > table > tbody > tr > td:first-child {
            border-left: none;
        }
        .table-container > table > tbody tr > td:last-child {
            border-right: none;
        }
        .table-container > table > tbody > tr:last-child > td {
            border-bottom: none;
        }
        /* Tighten gaps, expand to fill container */
        .table-container > table {
            border-collapse: collapse;
            width: 100%;
        }
        /* Add internal padding */
        .table-container > table > tbody > tr > td {
            padding: 10px 20px;
            border: 1px solid var(--graphical-fg);
        }


        /* Fix input placeholder text alignment */
        input::placeholder {
            text-align: left !important;
        }
    </style>
    {% block head %}{% endblock head %}
</head>
<body>
<header>
<nav class="container">
    <a href="/analysis" class="logo">SonnyAI</a>
</div>
</header>
{% block body %}{% endblock body %}
</body>
"#;

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}


