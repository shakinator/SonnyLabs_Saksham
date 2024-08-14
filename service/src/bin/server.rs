#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    sonnylabs::serve().await
}
