use sqlx::PgPool;

pub async fn connect() -> PgPool {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL environment should be set");

    PgPool::connect(&database_url).await.unwrap()
}
