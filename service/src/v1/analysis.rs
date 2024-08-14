use axum::{
    extract::{Path, Query, State, Extension},
    response::Json,
    routing::post,
};
use axum_extra::routing::RouterExt;
use serde::{Deserialize, Serialize};
use tracing::log::*;

use crate::{AppResult, AppRouter, auth};

pub fn router() -> AppRouter {
    AppRouter::new().route_with_tsr(
        "/v1/analysis/:analysis_id",
        post(post_analysis_item_endpoint),
    )
}

#[derive(Deserialize)]
struct PostAnalysisItemPath {
    analysis_id: i64,
}
#[derive(Deserialize)]
struct PostAnalysisItemQuery {
    tag: String,
    capture: Option<bool>,
}

#[derive(Serialize)]
struct PostAnalysisItemResponse {
    analysis: Vec<PostAnalysisItem>,
}
#[derive(Serialize)]
#[serde(tag = "type")]
enum PostAnalysisItem {
    #[serde(rename = "score")]
    Score {
        name: String,
        result: f64,
    },
    #[serde(rename = "PII")]
    PII {
        name: String,
        result: Vec<PostPIIResult>,
    }
}
#[derive(Serialize)]
struct PostPIIResult {
    text: String,
    label: String,
}
async fn post_analysis_item_endpoint(
    Path(PostAnalysisItemPath { analysis_id }): Path<PostAnalysisItemPath>,
    Query(PostAnalysisItemQuery { tag, capture }): Query<PostAnalysisItemQuery>,
    State(pool): State<sqlx::PgPool>,
    Extension(claims): Extension<auth::Claims>,
    body: String,
) -> AppResult<Json<PostAnalysisItemResponse>> {
    // Run prompt_injection tests
    let pi_score = crate::ml::prompt_injection::score_prompt_injection(body.clone())
        .await
        .unwrap();
    info!("pi badness: {pi_score}");

    let toxicity_score = crate::ml::toxicity::score_toxicity(body.clone()).await.unwrap();
    info!("toxicity score: {toxicity_score}");

    let pii = crate::ml::pii::extract_pii(body).await.unwrap();

    if capture.unwrap_or(true) {
        sqlx::query! {
            r#"
            INSERT INTO analysis_item
                (analysis_id, tag, measurement_type_id, confidence)
            SELECT analysis.id, $3, $4::INTEGER, $5::REAL
            FROM analysis
            WHERE analysis.id = $2 AND analysis.owner_id = $1
            UNION
            SELECT analysis.id, $3, $6::INTEGER, $7::REAL
            FROM analysis
            WHERE analysis.id = $2 AND analysis.owner_id = $1
            "#,
            claims.sub,
            analysis_id, tag,
            1, pi_score as f32,
            2, toxicity_score as f32,
        }
        .execute(&pool)
        .await
        .unwrap();
    }

    Ok(PostAnalysisItemResponse {
        analysis: vec![
            PostAnalysisItem::Score {
                name: "prompt_injection".into(),
                result: pi_score
            },
            PostAnalysisItem::Score {
                name: "toxicity".into(),
                result: toxicity_score
            },
            PostAnalysisItem::PII {
                name: "PII".into(),
                result: pii
                    .into_iter()
                    .map(|v| PostPIIResult { text: v.text, label: v.label })
                    .collect(),
            }
        ]
    }
    .into())
}
