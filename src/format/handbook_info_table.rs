use serde::de::{Deserialize, Deserializer};

use crate::format::*;
use crate::game_data::{OperatorFile, OperatorFileEntry, OperatorFileUnlock};

use std::collections::HashMap;

impl DataFile for HandbookInfoTable {
  const LOCATION: &'static str = "excel/handbook_info_table.json";
  const IDENTIFIER: &'static str = "handbook_info_table";
}

#[repr(transparent)]
#[derive(Debug, Clone, Deserialize)]
pub(super) struct HandbookInfoTable {
  #[serde(rename = "handbookDict")]
  handbook_dict: HashMap<String, HandbookInfoTableEntry>
}

impl HandbookInfoTable {
  pub(super) fn take_operator_file(&mut self, id: &str) -> Option<OperatorFile> {
    self.handbook_dict.remove(id).map(HandbookInfoTableEntry::into_operator_file)
  }
}

#[derive(Debug, Clone, Deserialize)]
struct HandbookInfoTableEntry {
  #[serde(rename = "charID")]
  char_id: String,
  #[serde(rename = "drawName")]
  illustrator_name: String,
  #[serde(rename = "storyTextAudio")]
  story_entries: Vec<HandbookStoryEntry>
}

impl HandbookInfoTableEntry {
  fn into_operator_file(self) -> OperatorFile {
    OperatorFile {
      id: self.char_id,
      illustrator_name: self.illustrator_name,
      entries: recollect(self.story_entries, HandbookStoryEntry::into_operator_file_entry)
    }
  }
}

#[derive(Debug, Clone, Deserialize)]
struct HandbookStoryEntry {
  stories: [HandbookStory; 1],
  #[serde(rename = "storyTitle")]
  story_title: String
}

impl HandbookStoryEntry {
  fn into_operator_file_entry(self) -> OperatorFileEntry {
    let HandbookStoryEntry { stories: [
      HandbookStory { story_text, unlock_type, unlock_param }
    ], story_title } = self;

    let unlock_condition = unlock_param.into_operator_file_unlock(unlock_type);
    OperatorFileEntry {
      title: story_title,
      text: story_text,
      unlock_condition
    }
  }
}

#[derive(Debug, Clone, Deserialize)]
struct HandbookStory {
  #[serde(rename = "storyText")]
  story_text: String,
  #[serde(rename = "unLockType")]
  unlock_type: u32,
  #[serde(rename = "unLockParam")]
  unlock_param: HandbookStoryUnlockParam
}

#[derive(Debug, Clone)]
enum HandbookStoryUnlockParam {
  // unlock_type: 0
  Always,
  // unlock_type: 1
  CharCondition(CharCondition),
  // unlock_type: 2
  Trust(u32),
  Other(String)
}

impl HandbookStoryUnlockParam {
  fn into_operator_file_unlock(self, unlock_type: u32) -> OperatorFileUnlock {
    match self {
      HandbookStoryUnlockParam::Always => {
        OperatorFileUnlock::AlwaysUnlocked
      },
      HandbookStoryUnlockParam::CharCondition(cond) => {
        OperatorFileUnlock::PromotionLevel(cond.into_promotion_and_level())
      },
      HandbookStoryUnlockParam::Trust(trust) => {
        OperatorFileUnlock::Trust(trust)
      },
      HandbookStoryUnlockParam::Other(char_id) if unlock_type == 6 => {
        OperatorFileUnlock::OperatorUnlocked(char_id)
      },
      HandbookStoryUnlockParam::Other(_) => {
        OperatorFileUnlock::AlwaysUnlocked
      }
    }
  }
}

impl<'de> Deserialize<'de> for HandbookStoryUnlockParam {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    #[inline]
    fn try_parse_trust(v: &str) -> Option<u32> {
      v.parse().ok()
    }

    #[inline]
    fn try_parse_char_condition(v: &str) -> Option<(CharPhase, u32)> {
      v.split_once(';').and_then(|(phase, level)| {
        let phase = phase.parse().ok().and_then(CharPhase::from_u32);
        let level = level.parse().ok();
        Option::zip(phase, level)
      })
    }

    struct HandbookStoryUnlockParamVisitor;

    impl HandbookStoryUnlockParamVisitor {
      fn visit<E>(self, v: Cow<str>) -> Result<HandbookStoryUnlockParam, E>
      where E: serde::de::Error {
        if v.is_empty() {
          Ok(HandbookStoryUnlockParam::Always)
        } else if let Some(trust) = try_parse_trust(&v) {
          Ok(HandbookStoryUnlockParam::Trust(trust))
        } else if let Some((phase, level)) = try_parse_char_condition(&v) {
          Ok(HandbookStoryUnlockParam::CharCondition(CharCondition { phase, level }))
        } else {
          Ok(HandbookStoryUnlockParam::Other(v.into_owned()))
        }
      }
    }

    impl<'de> serde::de::Visitor<'de> for HandbookStoryUnlockParamVisitor {
      type Value = HandbookStoryUnlockParam;

      #[inline]
      fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str({
          "an empty string, an integer literal, two integer literals delimited by a semicolon, or a character id"
        })
      }

      #[inline]
      fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
      where E: serde::de::Error {
        self.visit(Cow::Borrowed(v))
      }

      #[inline]
      fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
      where E: serde::de::Error {
        self.visit(Cow::Borrowed(v))
      }

      #[inline]
      fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
      where E: serde::de::Error {
        self.visit(Cow::Owned(v))
      }
    }

    deserializer.deserialize_string(HandbookStoryUnlockParamVisitor)
  }
}
