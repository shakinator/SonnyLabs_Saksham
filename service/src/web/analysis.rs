use axum::{
    extract::{Path, Query, State, Form, Extension},
    http::{status::StatusCode, header::HeaderMap},
    response::{Html, Redirect},
    routing::{get},
};
use axum_extra::routing::RouterExt;
use minijinja::context;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use tracing::log::*;

use crate::{
    AppError, AppRouter, MJEnvironment,
    header::into_headers,
    auth::Claims,
};

pub fn add_templates(mj_environment: &mut MJEnvironment) {
    mj_environment
        .add_template("analysis::index", ANALYSIS_INDEX_TEMPLATE)
        .unwrap();
    mj_environment
        .add_template("analysis::create", ANALYSIS_CREATE_TEMPLATE)
        .unwrap();
    mj_environment
        .add_template("analysis::show", ANALYSIS_TEMPLATE)
        .unwrap();
    mj_environment
        .add_template("analysis::graph::scatter", SCATTER_GRAPH)
        .unwrap();
    mj_environment
        .add_template("analysis::graph::exceedance", EXCEEDANCE_GRAPH)
        .unwrap();
}

pub fn router() -> AppRouter {
    AppRouter::new()
        .route_with_tsr("/analysis", get(analysis_index_endpoint))
        .route_with_tsr("/analysis/new", get(get_analysis_create_endpoint).post(post_analysis_create_endpoint))
        .route_with_tsr("/analysis/:analysis_id", get(analysis_endpoint))
        .route_with_tsr(
            "/analysis/:analysis_id/instances",
            get(analysis_instances_endpoint),
        )
        .route_with_tsr("/analysis/:analysis_id/graph/scatter", get(scatter_graph))
        .route_with_tsr(
            "/analysis/:analysis_id/graph/exceedance",
            get(exceedance_graph),
        )
}

const ANALYSIS_INDEX_TEMPLATE: &'static str = r#"
{% extends "base.html" %}
{% block title %}Available Analysis | SonnyAI{% endblock title %}
{% block head %}
<style type="text/css">
#analysis_summary {
    margin-top: var(--gap);
}
</style>
{% endblock head %}
{% block body %}
<div class="container">
<h1>Available Analysis</h1>

<a class="<button>" href="/analysis/new">
    + Create New Analysis
</a>

<div id=analysis_summary class=table-container>
    <table>
        <thead>
            <tr>
                <th>Id</th>
                <th>Analysis Name</th>
                <th>Total Calls</th>
                <th>Total Threats</th>
            </tr>
        </thead>
        <tbody>
            {% for item in analysis %}
            <tr>
                <td>{{item.id}}</td>
                <td><a href="{{item.url}}">{{item.name}}</a></td>
                <td>{{item.total}}</td>
                <td>{{item.nthreats}}</td>
            </tr>
            {% endfor %}
        <tbody>
    </table>
</div>
</div>
{% endblock body %}
"#;

async fn analysis_index_endpoint(
    Extension(claims): Extension<Claims>,
    State(mj_env): State<MJEnvironment>,
    State(pool): State<sqlx::PgPool>,
) -> Html<String> {
    #[derive(Serialize)]
    struct Analysis {
        id: i64,
        name: String,
        url: String,
        total: i64,
        nthreats: i64,
    }

    let mut analysis_list = Vec::new();

    let analysis_names = sqlx::query! {
        "
        SELECT id, name
        FROM analysis
        WHERE owner_id = $1
        ORDER BY id DESC
        ",
        claims.sub
    }
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.id, row.name))
    .collect::<HashMap<_, _>>();

    let analysis_totals = sqlx::query! {
        "
        SELECT analysis_id, count(*) as count
        FROM analysis_item
        JOIN analysis ON analysis.id = analysis_item.analysis_id
        WHERE analysis.owner_id = $1
        GROUP BY analysis_id
        ",
        claims.sub
    }
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.analysis_id, row.count))
    .collect::<HashMap<_, _>>();

    let analysis_attacks = sqlx::query! {
        "
        SELECT analysis_id, count(*) as count
        FROM analysis_item
        JOIN analysis ON analysis.id = analysis_item.analysis_id
        WHERE
            confidence > 0.5
            AND owner_id = $1
        GROUP BY analysis_id
        ",
        claims.sub
    }
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.analysis_id, row.count))
    .collect::<HashMap<_, _>>();

    for (id, name) in analysis_names.into_iter() {
        analysis_list.push(Analysis {
            id,
            name,
            url: format!("/analysis/{id}"),
            total: analysis_totals.get(&id).unwrap_or(&Some(0)).unwrap_or(0),
            nthreats: analysis_attacks.get(&id).unwrap_or(&Some(0)).unwrap_or(0),
        });
    }
    analysis_list.sort_by(|a, b| a.id.partial_cmp(&b.id).unwrap());

    Html(
        mj_env
            .get_template("analysis::index")
            .unwrap()
            .render(context!(analysis => analysis_list))
            .unwrap(),
    )
}


const ANALYSIS_CREATE_TEMPLATE: &'static str = r##"
{% extends "base.html" %}
{% block title %}New Analysis | SonnyAI{% endblock title %}
{% block head %}
<style type="text/css">
    input[type="text"] {
        width: 100%;
    }
    form > * {
        margin-bottom: var(--gap);
    }
    form.htmx-request {
        //background: red;
    }
</style>
{% endblock head %}
{% block body %}
<div class="container">
<h1>New Analysis</h1>

{% block form %}
<form
    hx-post="/analysis/new"
    hx-indicator="this"
    hx-disabled-elt="#submit"
    hx-swap="outerHTML"
>
    <input
        name="name"
        type="text"
        value="{{value}}"
        placeholder="e.g. Customer Queries Chatbot"
    >

    <button type="submit" id="submit">Submit</button>

    {% if success %}
        <div>Created Analysis. Redirecting...</div>
    {% endif %}

</form>
{% endblock form %}

</div>
{% endblock body %}
"##;

#[axum::debug_handler(state = crate::AppState)]
async fn get_analysis_create_endpoint(
    State(mj_env): State<MJEnvironment>,
) -> Html<String> {
    Html(
        mj_env
            .get_template("analysis::create").unwrap()
            .render(context!(value => "")).unwrap()
    )
}

#[derive(Deserialize, Debug)]
struct AnalysisCreatePayload {
    name: String,
}


async fn post_analysis_create_endpoint(
    Extension(claims): Extension<Claims>,
    State(pool): State<sqlx::PgPool>,
    Form(body): Form<AnalysisCreatePayload>,
) -> (StatusCode, HeaderMap) {
    let res = sqlx::query!{
        "
        INSERT INTO analysis (name, owner_id)
        VALUES ($1, $2)
        RETURNING id
        ",
        body.name, claims.sub
    }
    .fetch_one(&pool)
    .await
    .unwrap();

    (
        StatusCode::CREATED,
        into_headers(&[
            (b"hx-redirect", &format!("/analysis/{}", res.id)),
            (b"location", &format!("/analysis/{}", res.id)),
        ]).unwrap(),
    )
}


const ANALYSIS_TEMPLATE: &'static str = r#"
{% extends "base.html" %}
{% block title %}{{analysis.name}} | Analysis | SonnyAI{% endblock title %}
{% block head %}
<script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
<style type="text/css">
#instance-graphs {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
}
#instance-graphs .graph {
    position: relative;
    width: 100%;
    height: 100%;
}

.graph canvas {
    width: 100% !important;
    height: 100% !important;
}
</style>
{% endblock head %}
{% block body %}
<div class="container">
<h1>{{analysis.name}}</h1>
<span class="chip info">Total: {{analysis.total}}</span>
<span class="chip info">Threats: {{analysis.nthreats}}</span>
<div style="margin-top: 20px;">
    <div role="tablist" aria-label="Tabs example">
      <button role="tab" aria-controls="instance-graphs-tab" aria-selected="true"
        >Graph</button>
      <button role="tab" aria-controls="instance-table"
        >Table</button>
    </div>

    <div id="instance-graphs-tab" role="tabpanel">
        <h2>Graph</h2>
        <div id="instance-graphs">
            <div
                hx-trigger="intersect once"
                hx-get="/analysis/{{analysis.id}}/graph/scatter"
                hx-swap="self"
            ></div>
            <div
                hx-trigger="intersect once"
                hx-get="/analysis/{{analysis.id}}/graph/exceedance"
                hx-swap="self"
            ></div>
        </div>
    </div>

    <div id="instance-table" role="tabpanel" hidden="true">
    <h2>Threats Detected</h2>
    <div class=table-container>
    <table id=analysis_instances style="table-layout: fixed">
        <thead>
            <tr>
                <th>Timestamp</th>
                <th>Tag</th>
                <th>Threat</th>
                <th>Confidence</th>
            </tr>
        </thead>
        <tbody>
            {% block instances %}
            {% for item in analysis.instances %}
            <tr
                {%- if loop.last %}
                hx-trigger="intersect once"
                hx-get={{analysis.next_instances_url}}
                hx-swap="afterend"
                {%- endif %}
            >
                <td>{{item.timestamp}}</td>
                <td>{{item.key}}</td>
                <td>{{item.threat}}</td>
                <td>{{item.confidence}}</td>
            </tr>
            {% endfor %}
            {% endblock instances %}
        </tbody>
    </table>
    </div>
    </div>
</div>
{% endblock body %}
"#;

#[derive(Serialize)]
struct AnalysisInstance {
    timestamp: String,
    key: String,
    threat: String,
    confidence: String,
}

#[derive(Deserialize)]
struct AnalysisParams {
    analysis_id: i64,
}

async fn analysis_endpoint(
    Path(AnalysisParams { analysis_id }): Path<AnalysisParams>,
    Extension(claims): Extension<Claims>,
    State(mj_env): State<MJEnvironment>,
    State(pool): State<sqlx::PgPool>,
) -> Result<Html<String>, Redirect> {
    let analysis_name = sqlx::query! {
        "
        SELECT name
        FROM analysis
        WHERE
            id = $1
            AND owner_id = $2
        ",
        analysis_id, claims.sub
    }
    .fetch_optional(&pool)
    .await
    .unwrap();

    let Some(analysis_name) = analysis_name else {
        info!("Analysis not found or not owned");
        return Err(Redirect::to("/analysis"));
    };

    let analysis_total = sqlx::query! {
        "
        SELECT count(*) as count
        FROM analysis_item
        WHERE analysis_id = $1
        ",
        analysis_id
    }
    .fetch_one(&pool)
    .await
    .unwrap();

    let analysis_nthreats = sqlx::query! {
        "SELECT count(*) as count FROM analysis_item
         WHERE analysis_id = $1
         AND confidence > 0.5",
        analysis_id
    }
    .fetch_one(&pool)
    .await
    .unwrap();

    let instances = sqlx::query! {
        "SELECT
            created_at, tag, measurement_type.name as threat, confidence
        FROM analysis_item
        JOIN measurement_type
            ON measurement_type.id = analysis_item.measurement_type_id
        WHERE analysis_id = $1
        ORDER BY created_at DESC, measurement_type.id ASC
        LIMIT $2",
        analysis_id, 100
    }
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| AnalysisInstance {
        timestamp: row.created_at.to_string(),
        key: row.tag,
        threat: row.threat,
        confidence: format!("{}%", row.confidence*100.0),
    })
    .collect::<Vec<_>>();

    let count = instances.len();
    let next_offset = count;

    Ok(Html(
        mj_env
            .get_template("analysis::show").unwrap()
            .render(context!(
                analysis => context!(
                    id => analysis_id,
                    name => analysis_name.name,
                    total => analysis_total.count,
                    nthreats => analysis_nthreats.count,
                    //instances => &instances[0..next_offset],
                    instances => instances,
                    next_instances_url => format!("/analysis/{analysis_id}/instances?offset={next_offset}")
                )
            )).unwrap()
    ))
}

#[derive(Deserialize)]
struct AnalysisInstanceQuery {
    offset: i64,
}
async fn analysis_instances_endpoint(
    Path(AnalysisParams { analysis_id }): Path<AnalysisParams>,
    Query(AnalysisInstanceQuery { offset }): Query<AnalysisInstanceQuery>,
    Extension(claims): Extension<Claims>,
    State(mj_env): State<MJEnvironment>,
    State(pool): State<sqlx::PgPool>,
) -> Html<String> {
    let instances = sqlx::query! {
        "SELECT
            created_at, tag, measurement_type.name, confidence
        FROM analysis_item
        JOIN analysis ON analysis.id = analysis_item.analysis_id
        JOIN measurement_type
            ON measurement_type.id = analysis_item.measurement_type_id
        WHERE
            analysis_id = $1
            AND owner_id = $4
        ORDER BY created_at DESC, measurement_type.id ASC
        LIMIT $2
        OFFSET $3",
        analysis_id, 100, offset, claims.sub
    }
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| AnalysisInstance {
        timestamp: row.created_at.to_string(),
        key: row.tag,
        threat: row.name,
        confidence: format!("{}%", row.confidence*100.0),
    })
    .collect::<Vec<_>>();

    let next_offset = offset + (instances.len() as i64);

    let r = mj_env.get_template("analysis::show").unwrap()
        .eval_to_state(context!(
            analysis => context!(
                instances => instances,
                next_instances_url => format!("/analysis/{analysis_id}/instances?offset={next_offset}"),
            )
        )).unwrap()
        .render_block("instances").unwrap();
    Html(r)
}

trait MinMax<T> {
    fn minmax(&mut self) -> Option<(T, T)>;
}

impl<T, U> MinMax<T> for U
where
    U: Iterator<Item = T>,
    T: Copy + std::cmp::PartialOrd,
{
    fn minmax(&mut self) -> Option<(T, T)> {
        let Some(first) = self.next() else {
            return None
        };

        let mut minmax = (first, first);
        while let Some(v) = self.next() {
            if v < minmax.0 {
                minmax.0 = v
            }
            if v > minmax.1 {
                minmax.1 = v
            }
        }

        Some(minmax)
    }
}

const SCATTER_GRAPH: &str = r#"
<div class="graph">
    <canvas id="scatter-graph"></canvas>
</div>
<script>
    new Chart( document.getElementById("scatter-graph"), {
        type: 'scatter',
        data: {
          label: 'Scatter dataset',
          datasets: {{datasets}},
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            scales: {
                y: {
                    title: {
                        display: true,
                        text: "% Confidence",
                    },
                    min: 0,
                    max: 100,
                },
                x: {
                    ticks: {
                        callback: function(value) {
                            const d = new Date(value*1000);
                            const yyyy = d.getUTCFullYear();
                            const mm = d.getUTCMonth().toString().padStart(2, "0");
                            const dd = d.getUTCDate().toString().padStart(2, "0");
                            const HH = d.getUTCHours().toString().padStart(2, "0");
                            const MM = d.getUTCMinutes().toString().padStart(2, "0");
                            const SS = d.getUTCSeconds().toString().padStart(2, "0");
                            return [`${yyyy}-${mm}-${dd}`,`${HH}:${MM}:${SS}Z`];
                        }
                    }
                }
            },
            plugins: {
                tooltip: {
                    callbacks: {
                        label: function(context) {
                            const d = new Date(context.raw.x*1000);
                            const yyyy = d.getUTCFullYear();
                            const mm = d.getUTCMonth().toString().padStart(2, "0");
                            const dd = d.getUTCDate().toString().padStart(2, "0");
                            const HH = d.getUTCHours().toString().padStart(2, "0");
                            const MM = d.getUTCMinutes().toString().padStart(2, "0");
                            const SS = d.getUTCSeconds().toString().padStart(2, "0");
                            isoDateTime = `${yyyy}-${mm}-${dd} ${HH}:${MM}:${SS}Z`;

                            return [`${context.dataset.label}: ${context.raw.y}%`, isoDateTime];
                        }
                    }
                }
            }
        }
    });
</script>
"#;

#[derive(Deserialize)]
struct ScatterGraphPath {
    analysis_id: i64,
}
async fn scatter_graph(
    Path(ScatterGraphPath { analysis_id }): Path<ScatterGraphPath>,
    Extension(claims): Extension<Claims>,
    State(mj_environment): State<MJEnvironment>,
    State(pool): State<sqlx::PgPool>,
) -> Result<Html<String>, AppError> {
    let res = sqlx::query! {
        "
        SELECT
            created_at, confidence, measurement_type.name
        FROM analysis_item
        JOIN analysis ON analysis.id = analysis_item.analysis_id
        JOIN measurement_type
            ON measurement_type.id = analysis_item.measurement_type_id
        WHERE
            analysis_id = $1
            AND owner_id = $2
        ORDER BY created_at DESC, measurement_type.id ASC
        LIMIT 1000
        ",
        analysis_id, claims.sub
    }
    .fetch_all(&pool)
    .await
    .unwrap();

    #[derive(Serialize)]
    struct DataItem {
        x: i64,
        y: i64,
    }

    let mut datasets = HashMap::<&str, Vec<DataItem>>::new();

    for row in res.iter() {
        let data_item = DataItem {
            x: row.created_at.timestamp(),
            y: (row.confidence * 100.0) as i64,
        };

        if let Some(data) = datasets.get_mut(row.name.as_str()) {
            data.push(data_item);
        } else {
            datasets.insert(row.name.as_str(), vec![data_item]);
        }
    }

    #[derive(Serialize)]
    struct DatasetItem<'a> {
        label: &'a str,
        data: Vec<DataItem>,
        #[serde(rename = "borderWidth")]
        border_width: i32,
    }
    let mut datasets = datasets
        .into_iter()
        .map(|(k, v)| DatasetItem {
            label: k,
            data: v,
            border_width: 1,
        })
        .collect::<Vec<_>>();
    datasets.sort_by(|a, b| a.label.partial_cmp(b.label).unwrap());

    Ok(Html(
        mj_environment
            .get_template("analysis::graph::scatter")
            .unwrap()
            .render(context!(
                datasets => datasets,
            ))
            .unwrap(),
    ))
}

const EXCEEDANCE_GRAPH: &str = r#"
<div class="graph" style="position: relative; width: 100%; height: 100%">
    <canvas id="exceedance-graph"></canvas>
</div>
<script>
     new Chart( document.getElementById("exceedance-graph"), {
         type: 'bar',
         data: {
           labels: {{labels}},
           datasets: {{datasets}},
         },
         options: {
             responsive: true,
             maintainAspectRatio: false,
             plugins: {
                 title: {
                     display: true,
                     text: '#Threats by Severity',
                 },
             },
             scales: {
                x: { stacked: true },
                y: { stacked: true },
             },
         }
     });
</script>
"#;

#[derive(Deserialize)]
struct ExceedanceGraphPath {
    analysis_id: i64,
}
async fn exceedance_graph(
    Path(ExceedanceGraphPath { analysis_id }): Path<ExceedanceGraphPath>,
    Extension(claims): Extension<Claims>,
    State(mj_environment): State<MJEnvironment>,
    State(pool): State<sqlx::PgPool>,
) -> Result<Html<String>, AppError> {
    let res = sqlx::query! {
        "
        SELECT
            created_at, confidence, measurement_type.name
        FROM analysis_item
        JOIN analysis ON analysis.id = analysis_item.analysis_id
        JOIN measurement_type
            ON measurement_type.id = analysis_item.measurement_type_id
        WHERE
            analysis_id = $1
            AND owner_id = $2
        ORDER BY created_at DESC
        LIMIT 10000
        ",
        analysis_id, claims.sub
    }
    .fetch_all(&pool)
    .await
    .unwrap();

    let buckets = [0.0, 0.25, 0.50, 0.75];

    let mut counts = HashMap::<&str, Vec<i32>>::new();

    for row in res.iter() {
        for i in 0..buckets.len() {
            if row.confidence >= buckets[i] && row.confidence < *buckets.get(i + 1).unwrap_or(&1.0)
            {
                if let Some(ds_counts) = counts.get_mut(row.name.as_str()) {
                    ds_counts[i] += 1;
                } else {
                    let mut ds_counts = vec![0; buckets.len()];
                    ds_counts[i] += 1;
                    counts.insert(row.name.as_str(), ds_counts);
                }
            }
        }
    }

    let mut labels = counts.keys().collect::<Vec<_>>();
    labels.sort();

    let green = (181.0 + 360.0, 39.6, 58.4);
    let red = (349.0, 79.0, 68.2);
    let c = |t: f64| {
        format!(
            "hsl({:.0},{:.0}%,{:.0}%)",
            green.0 * (1.0 - t) + red.0 * t,
            green.1 * (1.0 - t) + red.1 * t,
            green.2 * (1.0 - t) + red.2 * t,
        )
    };
    let colors = [c(0.35), c(0.6), c(0.85), c(1.0)];
    assert!(colors.len() == buckets.len());

    #[derive(Serialize)]
    struct DataSet {
        label: String,
        data: Vec<i32>,
        #[serde(rename = "backgroundColor")]
        background_color: String,
    }
    let mut datasets = Vec::new();
    for i in 0..buckets.len() {
        datasets.push(DataSet {
            label: format!("{}%-{}%", i * 25, (i + 1) * 25),
            data: labels.iter().map(|k| counts.get(*k).unwrap()[i]).collect(),
            background_color: colors[i].clone(),
        })
    }

    Ok(Html(
        mj_environment
            .get_template("analysis::graph::exceedance")
            .unwrap()
            .render(context!(
                labels => labels,
                datasets => datasets,
            ))
            .unwrap(),
    ))
}
