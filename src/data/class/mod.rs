use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub mod db;

const CLASS_COLLECTION_NAME: &str = "classes";

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum ClassRole {
    Student,
    Assistant,
    Teacher,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClassParticipant {
    pub user_id: Uuid,
    pub class_role: ClassRole,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Class {
    #[serde(
        default = "Uuid::new_v4",
        rename = "_id",
        with = "bson::serde_helpers::uuid_1_as_binary"
    )]
    id: Uuid,
    name: String,

    #[serde(default)]
    participants: Vec<ClassParticipant>,
}
