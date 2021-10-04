use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Role {
    None,
    Normal,
    Author,
    Admin,
}

impl Into<u8> for Role {
    fn into(self) -> u8 {
        match self {
            Role::None => 0u8,
            Role::Normal => 1u8,
            Role::Author => 2u8,
            Role::Admin => 3u8,
        }
    }
}

impl From<u8> for Role {
    fn from(value: u8) -> Self {
        vec![Role::None, Role::Normal, Role::Author, Role::Admin][value as usize]
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
