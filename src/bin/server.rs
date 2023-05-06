use tracing::Level;

#[rocket::main]
async fn main() {

    #[cfg(debug_assertions)]
    let level = Some(Level::DEBUG);
    #[cfg(not(debug_assertions))]
    let level = Some(Level::INFO);

    let r = knowmark_backend::create(level).await;
    match r.launch().await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("Error launching server: {}", e);
        }
    };
}
