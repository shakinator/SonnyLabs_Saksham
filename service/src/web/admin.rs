use axum::{
    extract::{Path, Query, Form, State, Extension},
    http::status::StatusCode,
    response::{Html, Redirect},
    routing::{get,post,delete},
};
use axum_extra::routing::RouterExt;
use minijinja::context;
use serde::{Deserialize, Serialize};

use crate::{
    AppRouter, MJEnvironment, AppState,
    auth::Claims,
};

pub fn add_templates(mj_environment: &mut MJEnvironment) {
    mj_environment
        .add_template("admin::index", ADMIN_INDEX_TEMPLATE)
        .unwrap();
    mj_environment
        .add_template("admin::accounts::index", ADMIN_ACCOUNT_INDEX_TEMPLATE)
        .unwrap();
    mj_environment
        .add_template("admin::account", ADMIN_ACCOUNT_TEMPLATE)
        .unwrap();
}

pub fn router(state: AppState) -> AppRouter {
    AppRouter::new()
        .route_with_tsr("/admin", get(admin_index_endpoint))
        .route_with_tsr(
            "/admin/accounts",
            get(get_admin_account_index_endpoint)
            .post(post_admin_account_index_endpoint))
        .route_with_tsr("/admin/accounts/:account_id", get(get_admin_account_endpoint))
        .route_with_tsr("/admin/accounts/:account_id/session", post(create_session_endpoint))
        .route_with_tsr(
            "/admin/accounts/:account_id/session/:token",
            delete(delete_session_endpoint))
        .route_layer(axum::middleware::from_fn_with_state(
            state, require_admin_middleware
        ))
}

async fn require_admin_middleware(
    Extension(claims): Extension<Claims>,
    State(pool): State<sqlx::PgPool>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<impl axum::response::IntoResponse, StatusCode> {

    let admin_account = sqlx::query! {
        "SELECT 1 as _ignore
        FROM account
        WHERE username = 'admin' AND id = $1",
        claims.sub
    }
    .fetch_optional(&pool)
    .await
    .unwrap();

    if admin_account.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}


const ADMIN_INDEX_TEMPLATE: &'static str = r#"
{% extends "base.html" %}
{% block title %}Admin | SonnyAI{% endblock title %}
{% block head %}
{% endblock head %}
{% block body %}
<div class="container">
<a href="/admin/accounts">Accounts</a>
</div>
{% endblock body %}
"#;

async fn admin_index_endpoint(
    State(mj_env): State<MJEnvironment>,
) -> Html<String> {
    Html(
        mj_env
            .get_template("admin::index")
            .unwrap()
            .render(context!())
            .unwrap(),
    )
}


const ADMIN_ACCOUNT_INDEX_TEMPLATE: &'static str = r#"
{% extends "base.html" %}
{% block title %}Accounts | Admin | SonnyAI{% endblock title %}
{% block head %}
<style type="text/css">
input[type=text] {
    width: 100%;
}
</style>
{% endblock head %}
{% block body %}
<div class="container">
<form action="/admin/accounts" method=get> 
<fieldset>
<legend>Search</legend>
<input name="q" type=text placeholder="search username or email" value="{{q}}">
<input type=submit value="Submit">
</fieldset>
</form>

<div id=accounts class=table-container>
    <table>
        <thead>
            <tr>
                <th>Id</th>
                <th>Username</th>
                <th>Email</th>
                <th>Created At</th>
                <th>Updated At</th>
            </tr>
        </thead>
        <tbody>
            {% for acc in accounts %}
            <tr>
                <td><a href="/admin/accounts/{{acc.id}}">{{acc.id}}</a></td>
                <td>{{acc.username}}</td>
                <td>{{acc.email}}</td>
                <td>{{acc.created_at}}</td>
                <td>{{acc.updated_at}}</td>
            </tr>
            {% endfor %}
        </tbody>
    </table>
</div>
</div>
{% endblock body %}
"#;

#[derive(Deserialize)]
struct GetAdminAccountIndexQuery {
    q: Option<String>,
}
async fn get_admin_account_index_endpoint(
    Query(GetAdminAccountIndexQuery{q}): Query<GetAdminAccountIndexQuery>,
    State(mj_env): State<MJEnvironment>,
    State(pool): State<sqlx::PgPool>,
) -> Html<String> {

    #[derive(Serialize)]
    struct Account {
        id: i64,
        username: String,
        email: String,
        created_at: String,
        updated_at: String,
    }

    let query: &str = q.as_ref().map_or("%", |q| q.as_str());

    let accounts = sqlx::query! {
        "SELECT
            id, username, email, created_at, updated_at
        FROM account
        WHERE username like $1 OR email like $1",
        query
    }
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| Account {
        id: row.id,
        username: row.username,
        email: row.email,
        created_at: row.created_at.map_or("<NULL>".into(), |v| v.to_rfc3339()),
        updated_at: row.updated_at.map_or("<NULL>".into(), |v| v.to_rfc3339()),
    })
    .collect::<Vec<_>>();

    Html(
        mj_env
            .get_template("admin::accounts::index")
            .unwrap()
            .render(context!(
                accounts,
                q => q.unwrap_or("".into())
            ))
            .unwrap(),
    )
}

async fn post_admin_account_index_endpoint(
    _body: String
) -> String {
    "post admin account index".into()
}


const ADMIN_ACCOUNT_TEMPLATE: &'static str = r##"
{% extends "base.html" %}
{% block title %}Account - {{account.username}} | Admin | SonnyAI{% endblock title %}
{% block head %}
<style type="text/css">
input[type=text] {
    width: 100%;
}
td {
    min-width: 0;
    word-break: break-all;
}
td button {
    word-break: normal;
}
</style>
{% endblock head %}
{% block body %}
<div class="container">
<h1>Account: {{account.username}}</h1>

<h2>Info</h2>
<div id=account class=table-container>
    <table>
        <tbody>
            <tr>
                <th>Id</th>
                <td>{{account.id}}</td>
            </tr>
            <tr>
                <th>Username</th>
                <td>{{account.username}}</td>
            </tr>
            <tr>
                <th>Email</th>
                <td>{{account.email}}</td>
            </tr>
            <tr>
                <th>Created At</th>
                <td>{{account.created_at}}</td>
            </tr>
            <tr>
                <th>Updated At</th>
                <td>{{account.updated_at}}</td>
            </tr>
        </tbody>
    </table>
</div>

<h2>Sessions</h2>
<form
    hx-post="/admin/accounts/{{account.id}}/session"
    hx-target="#sessions table tbody"
    hx-swap=afterbegin
>
    <fieldset
        style="display: flex; flex-direction: row"
    >
        <legend>New Token</legend>
        <label for=is_api>API Token</label>
        <input type=checkbox name=is_api checked="checked" style="margin: 0 var(--gap)">
        <input type=submit value="New Token">
    </fieldset>
</form>
<div id=sessions class=table-container>
    <table>
        <thead>
            <tr>
                <th>Token</th>
                <th>Expires At</th>
                <th>Created At</th>
                <th></th>
            </tr>
        </thead>
        <tbody>
            {% for session in sessions %}
            {% block session_table_row %}
            <tr>
                <td>{{session.token}}</td>
                <td>{{session.expires_at}}</td>
                <td>{{session.created_at}}</td>
                <td>
                    <button
                        hx-delete="/admin/accounts/{{account.id}}/session/{{session.token}}"
                        hx-target="closest tr"
                        hx-swap=delete
                    >
                        Delete
                    </button>
                </td>
            </tr>
            {% endblock session_table_row %}
            {% endfor %}
        </tbody>
    </table>
</div>
</div>
{% endblock body %}
"##;

#[derive(Deserialize)]
struct GetAdminAccountParams {
    account_id: i64
}
async fn get_admin_account_endpoint(
    Path(GetAdminAccountParams{account_id}): Path<GetAdminAccountParams>,
    State(mj_env): State<MJEnvironment>,
    State(pool): State<sqlx::PgPool>,
) -> Html<String> {
    
    #[derive(Serialize)]
    struct Account {
        id: i64,
        username: String,
        email: String,
        created_at: String,
        updated_at: String,
    }

    let row = sqlx::query! {
        "SELECT
            id, username, email, created_at, updated_at
        FROM account
        WHERE id = $1",
        account_id
    }
    .fetch_one(&pool)
    .await
    .unwrap();
    let account = Account {
        id: row.id,
        username: row.username,
        email: row.email,
        created_at: row.created_at.map_or("<NULL>".into(), |v| v.to_rfc3339()),
        updated_at: row.updated_at.map_or("<NULL>".into(), |v| v.to_rfc3339()),
    };

    #[derive(Serialize)]
    struct Session {
        token: String,
        expires_at: String,
        created_at: Option<String>
    }

    let sessions = sqlx::query! {
        "SELECT
            token, expires_at, created_at
        FROM session
        WHERE account_id = $1
        ORDER BY created_at DESC",
        account_id
    }
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| Session {
        token: row.token,
        expires_at: row.expires_at.to_rfc3339(),
        created_at: row.created_at.map(|d| d.to_rfc3339())
    })
    .collect::<Vec<_>>();

    Html(
        mj_env
            .get_template("admin::account")
            .unwrap()
            .render(context!(
                account,
                sessions,
            ))
            .unwrap(),
    )
}

#[derive(Deserialize)]
struct CreateSessionPath {
    account_id: i64
}
#[derive(Deserialize)]
struct CreateSessionForm {
    is_api: Option<String>,
}
async fn create_session_endpoint(
    Path(CreateSessionPath{account_id}): Path<CreateSessionPath>,
    State(mj_env): State<MJEnvironment>,
    State(pool): State<sqlx::PgPool>,
    Form(CreateSessionForm{is_api}): Form<CreateSessionForm>,
) -> Html<String> {

    let token_prefix: &str = match is_api.as_deref() {
        Some("on") => "api::",
        _ => "",
    };

    use chrono::{Utc, TimeZone};
    let session = sqlx::query! {
        "
        INSERT INTO
            session(account_id, token, expires_at, login_info)
        VALUES ($1, CONCAT($2::TEXT, gen_random_uuid()::TEXT), $3, $4)
        RETURNING token, expires_at, created_at
        ",
        account_id,
        token_prefix,
        Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
        "Created in Admin panel"
    }
    .fetch_one(&pool)
    .await
    .unwrap();

    let r = mj_env.get_template("admin::account").unwrap()
        .eval_to_state(context!(
            account => context!(
                id => account_id
            ),
            session => context!(
                token => session.token,
                created_at => session.created_at.map(|d| d.to_rfc3339()),
                expires_at => session.expires_at.to_rfc3339(),
            )
        )).unwrap()
        .render_block("session_table_row").unwrap();
    Html(r.into())
}

#[derive(Deserialize)]
struct DeleteSessionPath {
    account_id: i64,
    token: String,
}
async fn delete_session_endpoint(
    Path(DeleteSessionPath{account_id, token}): Path<DeleteSessionPath>,
    State(pool): State<sqlx::PgPool>,
) -> StatusCode {
    let _ = sqlx::query! {
        "
        DELETE FROM session
        WHERE token = $1 and account_id = $2
        ",
        token, account_id
    }
    .execute(&pool)
    .await
    .unwrap();

    return StatusCode::OK;
}
