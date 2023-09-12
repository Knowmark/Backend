use std::ffi::OsString;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
#[cfg(target_os = "windows")]
use windows_service::{define_windows_service, service_dispatcher};

#[cfg(target_os = "windows")]
define_windows_service!(ffi_service_main, knowmark_service_main);

fn knowmark_service_main(arguments: Vec<OsString>) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::TRACE)
            .finish();

        if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
            eprintln!("{}", err);
        };

        let r = knowmark_backend::create(Some(Level::INFO)).await;
        let r = match r.launch().await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Error launching server: {}", e);
                return;
            }
        };
        // TODO: handle windows service stop signal
    });
}

#[cfg(target_os = "windows")]
fn main() -> Result<(), windows_service::Error> {
    // Register generated `ffi_service_main` with the system and start the service, blocking
    // this thread until the service is stopped.
    service_dispatcher::start("knowmark_service", ffi_service_main)?;
    Ok(())
}

#[cfg(not(target_os = "window"))]
fn main() {
    knowmark_service_main(std::env::args_os().collect());
}
