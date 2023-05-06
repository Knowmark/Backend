use base64::Engine;
use rocket::{Build, Rocket};

pub mod files;
pub mod quiz;
pub mod users;

use files::*;
use quiz::*;
use users::*;

pub fn mount_api(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .mount("/api/v1/user", routes![user_get, user_create, user_delete])
        .mount("/api/v1/login", routes![app, login_submit])
        .mount("/api/v1/api", routes![app])
        .mount(
            "/api/v1/quiz",
            routes![quiz_list, quiz_create, quiz_info, quiz_delete],
        )
        .mount("/", routes![app, app_path])
}
