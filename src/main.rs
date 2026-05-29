mod app;
mod device;
mod web;

#[tokio::main]
async fn main() {
    app::run().await;
}
