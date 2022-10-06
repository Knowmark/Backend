use derive_more::{Deref, DerefMut};
use std::cmp::Ordering;
use std::ops::Deref;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct BirthDate {
    pub month: u8,
    pub day: u8,
}
impl PartialOrd for BirthDate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for BirthDate {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.month.cmp(&other.month) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => self.day.cmp(&other.day),
            Ordering::Greater => Ordering::Greater,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deref)]
pub struct Tag(String);

impl<S: ToString> From<S> for Tag {
    fn from(s: S) -> Self {
        Tag(s.to_string())
    }
}

#[derive(Debug, Clone, Hash, Deref, DerefMut)]
pub struct Interests(Vec<Tag>);

#[derive(Debug, Clone, Hash)]
pub struct Profile {
    pub birth_date: BirthDate,
    pub interests: Interests,
}
