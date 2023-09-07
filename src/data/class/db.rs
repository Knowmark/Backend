use super::ClassRole;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ClassCreateData {
    pub name: String,
    pub owner: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AddUserData {
    pub class: Uuid,
    pub user: Uuid,
    pub role: ClassRole,
}
