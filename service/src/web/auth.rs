use axum::{
    extract::{State, Form, Query},
    http::{status::StatusCode, header::{HeaderMap}},
    response::Html,
    routing::get,
};
use axum_extra::{
    extract::cookie::CookieJar,
    routing::RouterExt
};
use serde::Deserialize;
use minijinja::context;

use tracing::log::*;

use crate::{
    AppRouter,
    MJEnvironment,
    auth::{Claims, Session},
    header::into_headers,
};

pub fn router() -> AppRouter {
    AppRouter::new()
        .route_with_tsr(
            "/login", get(get_login_endpoint).post(post_login_endpoint)
        )
}

pub fn add_templates(mj_environment: &mut MJEnvironment) {
    mj_environment
        .add_template("login", LOGIN_TEMPLATE)
        .unwrap();
}


const LOGIN_TEMPLATE: &'static str = r##"
{% extends "base.html" %}
{% block title %}Available Analysis | SonnyAI{% endblock title %}
{% block head %}
<script src="https://unpkg.com/htmx.org@1.9.10/dist/ext/response-targets.js"></script>
<style type="text/css">
#login {
    margin: 0 auto;
    display: inline-block;
}
#login > * {
    margin-bottom: calc(var(--gap) * 0.5);
}

#login-message {
    display: none;
    margin-left: var(--gap);
}
.htmx-request #login-message {
    display: inline;
}
.htmx-request #errors {
    display: none;
}
</style>

<script type="text/javascript">
function clearMessages() {
    document.getElementById('messages').replaceChildren();
}
</script>
{% endblock head %}

{% block body %}
<div class="container">
<h1>Login to Sonnylabs.ai</h1>

<form
    id="login"
    hx-post="/login"
    hx-indicator="this"
    hx-disabled-elt="#submit"

    hx-ext="response-targets"
    hx-target="#messages"
    hx-target-error="#messages"
    onsubmit="clearMessages()"
>
    <input
        name="username"
        type="text"
        placeholder="username or email"
    >
    <input
        name="password"
        type="password"
        placeholder="password"
    >

    {% if next %}
    <input
        name="next"
        hidden="hidden"
        value={{next}}
    >
    {% endif %}

    <div style="display: flex; justify-content: start; align-items: center">
        <input id="submit" type="submit" value="Login">
        <span id="login-message">logging in...</span>
    </div>

    <div id="messages">
        {% block messages %}
        {% for error in errors %}
            <div class="bad bg color border" style="padding: 0 var(--gap)">
                {{error}}
            </div>
        {% endfor %}
        {% endblock messages %}
    </div>
</form>

</div>
{% endblock body %}
"##;

#[derive(Deserialize)]
struct LoginQueryParams {
    next: Option<String>
}

async fn get_login_endpoint(
    Query(query): Query<LoginQueryParams>,
    State(mj_env): State<MJEnvironment>
) -> Html<String> {
    Html(
        mj_env
            .get_template("login")
            .unwrap()
            .render(context!(next => query.next))
            .unwrap()
    )
}

#[derive(Deserialize)]
struct LoginPayload {
    username: String,
    password: String,
    next: Option<String>,
}
async fn post_login_endpoint(
    headers: HeaderMap,
    State(mj_env): State<MJEnvironment>,
    State(pool): State<sqlx::PgPool>,
    jar: CookieJar,
    Form(body): Form<LoginPayload>
) -> (StatusCode, Option<CookieJar>, Option<HeaderMap>, Html<String>) {
    let render = |errors: &[&str]| {
        Html(
            mj_env.get_template("login").unwrap()
                .eval_to_state(context!(
                    errors => errors,
                )).unwrap()
                .render_block("messages").unwrap()
                .into()
        )
    };

    if body.username.len() == 0 {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            None, None,
            render(&["missing username or email"])
        );
    }
    if body.password.len() == 0 {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            None, None,
            render(&["missing password"])
        );
    }

    // check with db
    let account = sqlx::query! {
        "
        SELECT id, password
        FROM account
        WHERE
            username = $1
            OR email = $1
        ",
        body.username,
    }
    .fetch_optional(&pool)
    .await
    .unwrap();

    let standard_refusal = (
        StatusCode::UNAUTHORIZED,
        None, None,
        render(&["username or password incorrect"])
    );

    if account.is_none() {
        info!("username not found");
        return standard_refusal;
    }
    let account = account.unwrap();

    if account.password.is_none() {
        info!("non-loginnable user");
        return standard_refusal;
    }

    // TODO: always check password
    let pw_check = tokio::task::spawn_blocking(|| {
        bcrypt::verify(body.password, &account.password.unwrap())
    }).await.unwrap();

    let pw_check = match pw_check {
        Ok(ok) => ok,
        Err(err) => {
            error!("{:?}", err);
            false
        }
    };

    if !pw_check {
        info!("Incorrect password!");
        return standard_refusal;
    }


    // Create new session and auth
    let session = Session::new(account.id, pool).await;
    let session_cookie = session.to_cookie(headers.clone());
    let jar = jar.add(session_cookie);

    let claims = Claims::new_from_session(&session);
    let token_cookie = claims.to_cookie(headers).unwrap();
    let jar = jar.add(token_cookie);


    let redirect: &str = body.next
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("/");

    (
        StatusCode::OK,
        Some(jar),
        Some(into_headers(&[(b"hx-redirect", redirect)]).unwrap()),
        render(&[])
    )
}
