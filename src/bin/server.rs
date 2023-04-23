use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[rocket::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
        eprintln!("{}", err);
    };

    let r = knowmark_backend::create().await;
    match r.launch().await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("Error launching server: {}", e);
        }
    };
}
