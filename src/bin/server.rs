use tracing::Level;

#[rocket::main]
async fn main() {
    #[cfg(debug_assertions)]
    let level = Some(Level::DEBUG);
    #[cfg(not(debug_assertions))]
    let level = Some(Level::INFO);

    let r = match knowmark_backend::create(level).await {
        Ok(it) => it,
        Err(err) => {
            tracing::error!("Unable to initialize backend: {}", err);
            std::panic::panic_any(err);
        }
    };

    match r.launch().await {
        Ok(_) => {}
        Err(err) => {
            tracing::error!("Error launching server: {}", err);
            std::panic::panic_any(err);
        }
    };
}
