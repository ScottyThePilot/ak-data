use crate::format::*;
use crate::game_data::*;

use std::collections::HashMap;

impl DataFile for SkillTable {
  const LOCATION: &'static str = "excel/skill_table.json";
  const IDENTIFIER: &'static str = "skill_table";
}

pub(crate) type SkillTable = HashMap<String, SkillTableEntry>;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SkillTableEntry {
  levels: Vec<SkillTableLevel>
}

impl SkillTableEntry {
  pub(super) fn split_levels(&self) -> Option<(&[SkillTableLevel; 7], Option<&[SkillTableLevel; 3]>)> {
    if self.levels.len() < 7 { return None };
    let (start, end) = self.levels.split_at(7);
    let start: &[SkillTableLevel; 7] = start.try_into().ok()?;
    let end: Option<&[SkillTableLevel; 3]> = end.try_into().ok();
    Some((start, end))
  }

  pub(super) fn name_activation_recovery(&self) -> Option<(String, SkillActivation, SkillRecovery)> {
    all_equal(self.levels.iter().map(|level| {
      let activation = level.skill_type.into_activation();
      let recovery = level.sp_data.sp_type.into_recovery();
      (level.name.clone(), activation, recovery)
    }))
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct SkillTableLevel {
  name: String,
  #[serde(rename = "rangeId")]
  range_id: Option<String>,
  description: Option<String>,
  #[serde(rename = "skillType")]
  skill_type: SkillTableSkillType,
  // fields omitted: durationType
  #[serde(rename = "spData")]
  sp_data: SkillTableSpData,
  #[serde(rename = "prefabId")]
  prefab_key: Option<String>,
  duration: f32,
  blackboard: Vec<SkillTableBlackboardEntry>
}

impl SkillTableLevel {
  pub(super) fn into_skill_level(self) -> OperatorSkillLevel {
    let description = self.apply_blackboard();

    OperatorSkillLevel {
      description,
      attack_range_id: self.range_id,
      prefab_key: self.prefab_key,
      duration: self.duration,
      max_charge_time: self.sp_data.max_charge_time,
      sp_cost: self.sp_data.sp_cost,
      initial_sp: self.sp_data.init_sp,
      increment: self.sp_data.increment
    }
  }

  fn get_blackboard(&self) -> HashMap<String, f32> {
    self.blackboard.iter()
      .map(|blackboard_entry| (blackboard_entry.key.to_lowercase(), blackboard_entry.value))
      .chain(std::iter::once(("duration".to_owned(), self.duration)))
      .collect::<HashMap<String, f32>>()
  }

  fn apply_blackboard(&self) -> Option<String> {
    self.description.as_deref().and_then(|description| {
      if description != "-" {
        Some(apply_templates(description, self.get_blackboard()))
      } else {
        None
      }
    })
  }
}

#[derive(Debug, Clone, Deserialize)]
struct SkillTableSpData {
  #[serde(rename = "spType")]
  sp_type: SkillTableSpType,
  // fields omitted: levelUpCost
  #[serde(rename = "maxChargeTime")]
  max_charge_time: u32,
  #[serde(rename = "spCost")]
  sp_cost: u32,
  #[serde(rename = "initSp")]
  init_sp: u32,
  increment: f32
}

#[derive(Debug, Clone, Deserialize)]
struct SkillTableBlackboardEntry {
  key: String,
  value: f32
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum SkillTableSpType {
  AutoRecovery = 1,
  OffensiveRecovery = 2,
  DefensiveRecovery = 4,
  Passive = 8
}

impl SkillTableSpType {
  fn into_recovery(self) -> SkillRecovery {
    match self {
      SkillTableSpType::Passive => SkillRecovery::Passive,
      SkillTableSpType::AutoRecovery => SkillRecovery::AutoRecovery,
      SkillTableSpType::OffensiveRecovery => SkillRecovery::OffensiveRecovery,
      SkillTableSpType::DefensiveRecovery => SkillRecovery::DefensiveRecovery
    }
  }
}

impl_deserialize_uint_enum! {
  SkillTableSpType,
  SkillTableSpTypeVisitor,
  "a positive integer, one of 1, 2, 4 or 8",
  match {
    1 => SkillTableSpType::AutoRecovery,
    2 => SkillTableSpType::OffensiveRecovery,
    4 => SkillTableSpType::DefensiveRecovery,
    8 => SkillTableSpType::Passive
  }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum SkillTableSkillType {
  Passive = 0,
  Manual = 1,
  Auto = 2
}

impl SkillTableSkillType {
  fn into_activation(self) -> SkillActivation {
    match self {
      SkillTableSkillType::Passive => SkillActivation::Passive,
      SkillTableSkillType::Manual => SkillActivation::Manual,
      SkillTableSkillType::Auto => SkillActivation::Auto
    }
  }
}

impl_deserialize_uint_enum! {
  SkillTableSkillType,
  SkillTableSkillTypeVisitor,
  "a positive integer, one of 0, 1, or 2",
  match {
    0 => SkillTableSkillType::Passive,
    1 => SkillTableSkillType::Manual,
    2 => SkillTableSkillType::Auto
  }
}
