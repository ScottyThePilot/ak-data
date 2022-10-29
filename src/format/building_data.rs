use crate::format::*;
use crate::game_data::*;

use std::collections::HashMap;

impl DataFile for BuildingData {
  const LOCATION: &'static str = "excel/building_data.json";
  const IDENTIFIER: &'static str = "building_data";
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BuildingData {
  rooms: HashMap<String, BuildingDataRoom>,
  chars: HashMap<String, BuildingDataChar>,
  buffs: HashMap<String, BuildingDataBuff>
}

impl BuildingData {
  pub(super) fn into_buildings(self) -> HashMap<BuildingType, Building> {
    self.rooms.into_values()
      .map(|building_data_room| {
        (building_data_room.id.into_building_type(), building_data_room.into_building())
      })
      .collect()
  }

  pub(super) fn get_operator_base_skill(&self, id: &str) -> Vec<OperatorBaseSkill> {
    // if an operator can't be found, just return an empty array of base skills
    self.chars.get(id).map_or_else(Vec::new, |BuildingDataChar { buffs, .. }| {
      buffs.iter().filter_map(|buff| buff.to_operator_base_skill(self)).collect()
    })
  }
}

#[derive(Debug, Clone, Deserialize)]
struct BuildingDataRoom {
  id: BuildingDataRoomId,
  name: String,
  description: Option<String>,
  #[serde(deserialize_with = "deserialize_negative_int")]
  #[serde(rename = "maxCount")]
  max_count: Option<u32>,
  category: String,
  size: BuildingDataRoomSize,
  phases: Vec<BuildingDataRoomPhase>
}

impl BuildingDataRoom {
  fn into_building(self) -> Building {
    Building {
      building_type: self.id.into_building_type(),
      name: self.name,
      description: self.description,
      max_count: self.max_count,
      category: self.category,
      size: self.size.into(),
      upgrades: recollect(self.phases, BuildingDataRoomPhase::into_building_upgrade)
    }
  }
}

#[derive(Debug, Clone, Deserialize)]
struct BuildingDataRoomPhase {
  #[serde(rename = "unlockCondId")]
  unlock_condition: String,
  #[serde(rename = "buildCost")]
  build_cost: BuildingDataBuildCost,
  electricity: i32,
  #[serde(rename = "maxStationedNum")]
  station_capacity: u32,
  #[serde(rename = "manpowerCost")]
  manpower_cost: u32
}

impl BuildingDataRoomPhase {
  fn into_building_upgrade(self) -> BuildingUpgrade {
    BuildingUpgrade {
      unlock_condition: self.unlock_condition,
      construction_cost: ItemCost::convert(self.build_cost.items),
      construction_drones: self.build_cost.labor,
      power: self.electricity,
      operator_capacity: self.station_capacity,
      manpower_cost: self.manpower_cost
    }
  }
}

#[derive(Debug, Clone, Deserialize)]
struct BuildingDataBuildCost {
  items: Vec<ItemCost>,
  labor: u32
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct BuildingDataRoomSize {
  row: u32,
  col: u32
}

impl From<BuildingDataRoomSize> for (u32, u32) {
  // [Row, Col] format is [Y, X], this converts it to [X, Y]
  fn from(room_size: BuildingDataRoomSize) -> Self {
    (room_size.col, room_size.row)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
enum BuildingDataRoomId {
  #[serde(rename = "CONTROL")]
  ControlCenter,
  #[serde(rename = "POWER")]
  PowerPlant,
  #[serde(rename = "MANUFACTURE")]
  Factory,
  #[serde(rename = "TRADING")]
  TradingPost,
  #[serde(rename = "DORMITORY")]
  Dormitory,
  #[serde(rename = "WORKSHOP")]
  Workshop,
  #[serde(rename = "HIRE")]
  Office,
  #[serde(rename = "TRAINING")]
  TrainingRoom,
  #[serde(rename = "MEETING")]
  ReceptionRoom,
  #[serde(rename = "ELEVATOR")]
  Elevator,
  #[serde(rename = "CORRIDOR")]
  Corridor
}

impl BuildingDataRoomId {
  fn into_building_type(self) -> BuildingType {
    match self {
      BuildingDataRoomId::ControlCenter => BuildingType::ControlCenter,
      BuildingDataRoomId::PowerPlant => BuildingType::PowerPlant,
      BuildingDataRoomId::Factory => BuildingType::Factory,
      BuildingDataRoomId::TradingPost => BuildingType::TradingPost,
      BuildingDataRoomId::Dormitory => BuildingType::Dormitory,
      BuildingDataRoomId::Workshop => BuildingType::Workshop,
      BuildingDataRoomId::Office => BuildingType::Office,
      BuildingDataRoomId::TrainingRoom => BuildingType::TrainingRoom,
      BuildingDataRoomId::ReceptionRoom => BuildingType::ReceptionRoom,
      BuildingDataRoomId::Elevator => BuildingType::Elevator,
      BuildingDataRoomId::Corridor => BuildingType::Corridor
    }
  }
}



#[derive(Debug, Clone, Deserialize)]
struct BuildingDataChar {
  // omitted fields: charId
  #[serde(rename = "buffChar")]
  buffs: Vec<BuildingDataCharBuff>
}

#[repr(transparent)]
#[derive(Debug, Clone, Deserialize)]
struct BuildingDataCharBuff {
  #[serde(rename = "buffData")]
  phases: Vec<BuildingDataCharBuffPhase>
}

impl BuildingDataCharBuff {
  fn to_operator_base_skill(&self, building_data: &BuildingData) -> Option<OperatorBaseSkill> {
    if self.phases.is_empty() { return None };
    let phases = self.phases.iter()
      .map(|phase| phase.to_operator_base_skill_phase(building_data))
      .collect();
    Some(OperatorBaseSkill { phases })
  }
}

#[derive(Debug, Clone, Deserialize)]
struct BuildingDataCharBuffPhase {
  #[serde(rename = "buffId")]
  id: String,
  #[serde(rename = "cond")]
  condition: CharCondition
}

impl BuildingDataCharBuffPhase {
  fn to_operator_base_skill_phase(&self, building_data: &BuildingData) -> OperatorBaseSkillPhase {
    let BuildingDataCharBuffPhase { id, condition } = self;
    building_data.buffs[id].to_operator_base_skill_phase(condition.clone())
  }
}

#[derive(Debug, Clone, Deserialize)]
struct BuildingDataBuff {
  // omitted fields: buffId
  #[serde(rename = "buffName")]
  name: String,
  #[serde(rename = "sortId")]
  sort: u32,
  #[serde(rename = "buffCategory")]
  category: BuildingDataBuffCategory,
  #[serde(rename = "roomType")]
  room_type: BuildingDataRoomId
}

impl BuildingDataBuff {
  fn to_operator_base_skill_phase(&self, condition: CharCondition) -> OperatorBaseSkillPhase {
    OperatorBaseSkillPhase {
      name: self.name.clone(),
      condition: condition.into_promotion_and_level(),
      sort: self.sort,
      category: self.category.into_operator_base_skill_category(),
      building_type: self.room_type.into_building_type()
    }
  }
}

#[derive(Debug, Clone, Copy, Deserialize)]
enum BuildingDataBuffCategory {
  #[serde(rename = "FUNCTION")]
  Function,
  #[serde(rename = "RECOVERY")]
  Recovery,
  #[serde(rename = "OUTPUT")]
  Output
}

impl BuildingDataBuffCategory {
  fn into_operator_base_skill_category(self) -> OperatorBaseSkillCategory {
    match self {
      BuildingDataBuffCategory::Function => OperatorBaseSkillCategory::Function,
      BuildingDataBuffCategory::Recovery => OperatorBaseSkillCategory::Recovery,
      BuildingDataBuffCategory::Output => OperatorBaseSkillCategory::Output
    }
  }
}
