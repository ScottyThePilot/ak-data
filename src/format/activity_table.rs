use chrono::{DateTime, Utc};

use crate::format::*;
use crate::game_data::{Event, EventType};

use std::collections::HashMap;

impl DataFile for ActivityTable {
  const LOCATION: &'static str = "excel/activity_table.json";
  const IDENTIFIER: &'static str = "activity_table";
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ActivityTable {
  #[serde(rename = "basicInfo")]
  basic_info: HashMap<String, ActivityTableBasicInfoEntry>
}

impl ActivityTable {
  pub(super) fn into_events(self) -> Vec<Event> {
    recollect_filter(self.basic_info, |(_, basic_info_entry)| basic_info_entry.into_event())
  }
}

#[derive(Debug, Clone, Deserialize)]
struct ActivityTableBasicInfoEntry {
  id: String,
  #[serde(rename = "displayType")]
  kind: Option<ActivityTableBasicInfoKind>,
  name: String,
  #[serde(rename = "startTime")]
  #[serde(with = "chrono::serde::ts_seconds")]
  start_time: DateTime<Utc>,
  #[serde(rename = "endTime")]
  #[serde(with = "chrono::serde::ts_seconds")]
  end_time: DateTime<Utc>,
  #[serde(rename = "rewardEndTime")]
  #[serde(with = "chrono::serde::ts_seconds")]
  end_time_rewards: DateTime<Utc>,
  #[serde(rename = "isReplicate")]
  is_rerun: bool
}

impl ActivityTableBasicInfoEntry {
  fn into_event(self) -> Option<Event> {
    Some(Event {
      id: self.id,
      name: self.name,
      event_type: self.kind?.into_event_type(),
      open_time: self.start_time,
      close_time: self.end_time,
      close_time_rewards: self.end_time_rewards,
      is_rerun: self.is_rerun
    })
  }
}

#[repr(u8)]
#[derive(Debug, Clone, Deserialize)]
enum ActivityTableBasicInfoKind {
  // Also known as "Intermezzi".
  #[serde(rename = "BRANCHLINE")]
  Branchline,
  #[serde(rename = "SIDESTORY")]
  SideStory,
  // Also known as "Vignettes".
  #[serde(rename = "MINISTORY")]
  MiniStory
}

impl ActivityTableBasicInfoKind {
  fn into_event_type(self) -> EventType {
    match self {
      ActivityTableBasicInfoKind::Branchline => EventType::Intermezzi,
      ActivityTableBasicInfoKind::SideStory => EventType::SideStory,
      ActivityTableBasicInfoKind::MiniStory => EventType::Vignette
    }
  }
}
