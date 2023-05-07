use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, ToSchema)]
#[repr(u8)]
pub enum Role {
    None,
    Normal,
    Author,
    Admin,
}

impl Role {
    const ALL: &[Role] = &[Role::None, Role::Normal, Role::Author, Role::Admin];
}

impl From<u8> for Role {
    fn from(value: u8) -> Self {
        Role::ALL[value as usize]
    }
}

impl Role {
    /// Indicates whether user with role can create Quizzes
    pub fn can_author(self) -> bool {
        self >= Role::Author
    }
}

impl std::default::Default for Role {
    fn default() -> Self {
        Role::None
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::None => write!(f, "none"),
            Role::Normal => write!(f, "normal"),
            Role::Author => write!(f, "author"),
            Role::Admin => write!(f, "admin"),
        }
    }
}

impl std::convert::Into<String> for Role {
    fn into(self) -> String {
        self.to_string()
    }
}
