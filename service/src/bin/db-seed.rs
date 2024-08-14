use sonnylabs::db;
use sqlx::query;
use bcrypt;

const MAX_TOTAL_CALLS: u32 = 1000;

struct AnalysisSummary {
    name: String,
    total: u32,
    nthreats: u32,
}
fn _analysis_summary(analysis_id: u64) -> AnalysisSummary {
    use rand::{Rng, SeedableRng};
    let mut rng = rand::rngs::StdRng::seed_from_u64(analysis_id);

    let name_len = rng.gen_range(1..5);
    let name = lipsum::lipsum_words_with_rng(rng.clone(), name_len).replace('.', "");

    let total = rng.gen_range(0..MAX_TOTAL_CALLS);
    let nthreats = rng.gen_range(0..total);

    AnalysisSummary {
        name,
        total,
        nthreats,
    }
}

struct AnalysisInstance {
    timestamp: chrono::DateTime<chrono::Utc>,
    tag: String,
    threat_id: i32,
    confidence: f32,
}
fn _analysis_instances(analysis_id: u64, threats: &[i32]) -> Vec<AnalysisInstance> {
    let nthreats = _analysis_summary(analysis_id).nthreats;

    use rand::{Rng, SeedableRng};
    let mut rng = rand::rngs::StdRng::seed_from_u64(analysis_id);

    let mut instances = Vec::new();
    for _ in 0..nthreats {
        use chrono::TimeZone;
        let timestamp = chrono::Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()
            + chrono::Duration::minutes(rng.gen_range(0..525600));
        let comment_id_bytes: [u8; 16] = rng.gen();
        let comment_uuid = uuid::Uuid::from_bytes(comment_id_bytes);

        for threat in threats.iter() {
            let confidence: f32 = rng.gen_range(0.0..1.0);

            instances.push(AnalysisInstance {
                timestamp: timestamp,
                tag: format!("comment::{}", comment_uuid),
                threat_id: *threat,
                confidence,
            })
        }
    }
    instances.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap());

    instances
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let pool = db::connect().await;


    let password = bcrypt::hash("admin", bcrypt::DEFAULT_COST).unwrap();
    let res = query! {
        "
        WITH _inserted AS (
            INSERT INTO account(email, username, password)
            VALUES ('admin@sonnylabs.ai', 'admin', $1)
            ON CONFLICT DO NOTHING
            RETURNING id
        )
        SELECT id from _inserted
        UNION
        SELECT id from account WHERE username='admin'
        ",
        password
    }
    .fetch_one(&pool)
    .await
    .unwrap();

    let admin_id = res.id;


    let threats = &[1, 2];

    for analysis_id in 1..10 {
        let analysis_summary = _analysis_summary(analysis_id);
        let analysis_items = _analysis_instances(analysis_id, threats);

        let res = query! {
            "
                INSERT INTO analysis (name, owner_id)
                VALUES ($1, $2)
                RETURNING id
            ",
            analysis_summary.name, admin_id
        }
        .fetch_one(&pool)
        .await
        .unwrap();

        // New analysis id from db
        let analysis_id = res.id;

        for v in analysis_items.iter() {
            query! {
                "
                    INSERT INTO analysis_item
                        (analysis_id, tag, measurement_type_id, confidence, created_at)
                    VALUES($1, $2, $3, $4, $5)
                ",
                analysis_id, v.tag, v.threat_id, v.confidence, v.timestamp
            }
            .execute(&pool)
            .await
            .unwrap();
        }
    }

    println!("done");
}
