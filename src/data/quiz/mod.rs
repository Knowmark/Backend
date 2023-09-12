use chrono::{DateTime, Utc};
use std::{collections::HashMap, time::Duration};
use utoipa::ToSchema;
use uuid::Uuid;

pub static PART_COLLECTION_NAME: &str = "quiz.parts";
pub static PARTICIPANT_COLLECTION_NAME: &str = "participant";
pub static QUIZ_COLLECTION_NAME: &str = "quiz";

fn true_bool() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub enum QuestionKind {
    Bool { answer: bool },
    /*
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
    */
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
pub enum QuizPart {
    Content {
        #[serde(default = "Uuid::new_v4")]
        id: Uuid,
        title: String,
        text: String,
    },
    Question {
        #[serde(default = "Uuid::new_v4")]
        id: Uuid,
        text: String,
        kind: QuestionKind,

        #[serde(default)]
        time_limit: Option<Duration>,

        /// Allow partial answers
        #[serde(default)]
        partial: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "t", content = "value")]
pub enum AnswerChoice {
    Number(f64),
    Short(String),
    Long(String),
    FillIn(Vec<String>),
    Single(u8),
    Multiple(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuizParticipant {
    pub user_id: Uuid,
    pub started_on: DateTime<Utc>,
    #[serde(default)]
    pub choices: HashMap<Uuid, AnswerChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Quiz {
    #[serde(
        default = "Uuid::new_v4",
        rename = "_id",
        with = "bson::serde_helpers::uuid_1_as_binary"
    )]
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub desc: String,
    #[serde(
        default = "Uuid::new_v4",
        with = "bson::serde_helpers::uuid_1_as_binary"
    )] // TODO: Remove default
    pub author: Uuid,
    #[serde(default = "Utc::now")]
    pub created: DateTime<Utc>,
    pub parts: Vec<QuizPart>,

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
    pub participants: Vec<QuizParticipant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum PartAnswer {
    Bool { answer: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuizAnswers {
    pub user: Uuid,
    #[serde(default)]
    pub date: DateTime<Utc>,
    pub answers: HashMap<Uuid, PartAnswer>,
}

#[derive(Debug, Default, Serialize, ToSchema)]
pub struct ValidationResult {
    pub total_questions: usize,
    pub correct_answers: usize,
}

impl QuizAnswers {
    pub fn validate(&self, quiz: &Quiz) -> ValidationResult {
        let mut result = ValidationResult::default();
        for part in &quiz.parts {
            if let QuizPart::Question {
                id,
                text,
                kind,
                partial,
                ..
            } = part
            {
                result.total_questions += 1;
                match kind {
                    QuestionKind::Bool { answer } => {
                        if let Some(PartAnswer::Bool {
                            answer: user_answer,
                        }) = self.answers.get(id)
                        {
                            if user_answer == answer {
                                result.correct_answers += 1;
                            }
                        }
                    }
                }
            }
        }
        return result;
    }
}
