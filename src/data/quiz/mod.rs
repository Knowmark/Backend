use chrono::{DateTime, Utc};
use std::{collections::HashMap, time::Duration};
use utoipa::ToSchema;
use uuid::Uuid;

pub static PART_COLLECTION_NAME: &str = "quiz.parts";
pub static PARTICIPANT_COLLECTION_NAME: &str = "participants";
pub static QUIZ_COLLECTION_NAME: &str = "quiz";

fn true_bool() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum AnswerType {
    Bool,
    Number,
    Short,
    Long,
    FillIn,
    Match(Vec<(String, String)>),
    Single {
        options: Vec<String>,
        #[serde(default = "true_bool")]
        shuffle: bool,
    },
    Multiple {
        options: Vec<String>,
        #[serde(default = "true_bool")]
        shuffle: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum AnswerValidation {
    Bool {
        expected: bool,
    },
    Exact {
        #[serde(default)]
        case_sensitive: bool,
        expected: String,
    },
    NumberRange {
        min: f64,
        max: f64,
    },
    #[cfg(feature = "validation-regex")]
    Regex {
        #[serde(default)]
        case_sensitive: bool,
        expr: String,
    },
    Multiple {
        #[serde(default)]
        case_sensitive: bool,
        expected: Vec<String>,
    },
    External {
        // for running external, locally installed validation programs/scripts.
        // VULN: Possible code injection. Can't just insert answers.
        command: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum Part {
    Content {
        #[serde(default = "Uuid::new_v4")]
        id: Uuid,
        title: String,
        text: String,
    },
    Interact {
        #[serde(default = "Uuid::new_v4")]
        id: Uuid,
        text: String,
        ans: AnswerType,
        choices: Vec<AnswerChoice>,

        time_limit: Option<Duration>,

        value: f32,
        validation: Option<AnswerValidation>,
        partial: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "t", content = "value")]
pub enum AnswerChoice {
    Bool(bool),
    Number(f64),
    Short(String),
    Long(String),
    FillIn(Vec<String>),
    Single(u8),
    Multiple(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParticipantInfo {
    pub id: Uuid,
    #[serde(default = "Utc::now")]
    pub started_on: DateTime<Utc>,
    #[serde(default)]
    pub choices: HashMap<Uuid, AnswerChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Quiz {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default = "Uuid::new_v4")] // TODO: Remove default
    pub author: Uuid,
    #[serde(default = "Utc::now")]
    pub created: DateTime<Utc>,
    #[serde(default)]
    pub parts: Vec<Part>,

    #[serde(default)]
    pub time_limit: Option<Duration>,
    #[serde(default)]
    pub expect_focus: bool,
    #[serde(default)]
    pub show_answer: bool,
    #[serde(default = "true_bool")]
    pub show_results: bool,

    #[serde(default = "true_bool")]
    pub public: bool,
    #[serde(default)]
    pub open_on: Option<DateTime<Utc>>,
    #[serde(default)]
    pub close_on: Option<DateTime<Utc>>,
    #[serde(default)]
    pub begin_buffer: Option<Duration>,
    #[serde(default)]
    pub participants: Vec<String>,
}
