use crate::format::*;
use crate::game_data::{OperatorModule, OperatorModuleMission};

use std::collections::HashMap;

impl DataFile for EquipTable {
  const LOCATION: &'static str = "excel/uniequip_table.json";
  const IDENTIFIER: &'static str = "uniequip_table";
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct EquipTable {
  #[serde(rename = "equipDict")]
  equip_list: HashMap<String, EquipTableEquip>,
  #[serde(rename = "missionList")]
  mission_list: HashMap<String, EquipTableMission>,
  #[serde(rename = "charEquip")]
  character_equip_list: HashMap<String, Vec<String>>
}

impl EquipTable {
  pub(super) fn take_operator_modules(&mut self, id: &str) -> Option<Vec<OperatorModule>> {
    let character_equip_list = self.character_equip_list.remove(id)?;
    recollect_maybe(character_equip_list.iter().skip(1).cloned(), |character_equip_id| {
      self.equip_list.remove(&character_equip_id).and_then(|equip_table_equip| {
        equip_table_equip.into_operator_module(&self.mission_list)
      })
    })
  }
}

#[derive(Debug, Clone, Deserialize)]
struct EquipTableEquip {
  #[serde(rename = "uniEquipId")]
  id: String,
  #[serde(rename = "uniEquipName")]
  name: String,
  #[serde(rename = "uniEquipDesc")]
  description: String,
  #[serde(rename = "unlockEvolvePhase")]
  unlock_phase: EquipTablePhase,
  #[serde(rename = "unlockLevel")]
  unlock_level: u32,
  #[serde(rename = "unlockFavorPoint")]
  unlock_trust_points: u32,
  #[serde(rename = "missionList")]
  mission_list: Vec<String>,
  #[serde(rename = "itemCost")]
  item_cost: Option<Vec<ItemCost>>
}

impl EquipTableEquip {
  fn into_operator_module(self, mission_list: &HashMap<String, EquipTableMission>) -> Option<OperatorModule> {
    let missions = recollect_maybe(self.mission_list, |id| {
      mission_list.get(&id).map(|mission| (id, mission.clone().into_operator_module_mission()))
    })?;

    Some(OperatorModule {
      id: self.id,
      name: self.name,
      description: self.description,
      condition: PromotionAndLevel {
        promotion: self.unlock_phase.into_promotion(),
        level: self.unlock_level
      },
      required_trust: trust_points_to_percent(self.unlock_trust_points),
      upgrade_cost: ItemCost::convert(self.item_cost.unwrap_or_default()),
      missions
    })
  }
}

#[derive(Debug, Clone, Deserialize)]
struct EquipTableMission {
  #[serde(rename = "desc")]
  description: String,
  #[serde(rename = "uniEquipMissionSort")]
  sort: u32
}

impl EquipTableMission {
  fn into_operator_module_mission(self) -> OperatorModuleMission {
    OperatorModuleMission {
      description: self.description,
      sort: self.sort
    }
  }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum EquipTablePhase {
  Elite0 = 0,
  Elite1 = 1,
  Elite2 = 2
}

impl EquipTablePhase {
  fn into_promotion(self) -> Promotion {
    match self {
      EquipTablePhase::Elite0 => Promotion::None,
      EquipTablePhase::Elite1 => Promotion::Elite1,
      EquipTablePhase::Elite2 => Promotion::Elite2
    }
  }
}

impl_deserialize_uint_enum! {
  EquipTablePhase,
  EquipTablePhaseVisitor,
  "a positive integer, one of 0, 1, or 2",
  match {
    0 => EquipTablePhase::Elite0,
    1 => EquipTablePhase::Elite1,
    2 => EquipTablePhase::Elite2
  }
}

fn trust_points_to_percent(points: u32) -> u32 {
  match points {
    0..=7 => 0, 8..=15 => 1, 16..=27 => 2, 28..=39 => 3, 40..=55 => 4,
    56..=71 => 5, 72..=91 => 6, 92..=111 => 7, 112..=136 => 8, 137..=161 => 9,
    162..=191 => 10, 192..=221 => 11, 222..=254 => 12, 255..=287 => 13, 288..=324 => 14,
    325..=361 => 15, 362..=403 => 16, 404..=445 => 17, 446..=490 => 18, 491..=535 => 19,
    536..=585 => 20, 586..=635 => 21, 636..=690 => 22, 691..=745 => 23, 746..=803 => 24,
    804..=861 => 25, 862..=923 => 26, 924..=985 => 27, 986..=1051 => 28, 1052..=1117 => 29,
    1118..=1183 => 30, 1184..=1249 => 31, 1250..=1315 => 32, 1316..=1381 => 33, 1382..=1456 => 34,
    1457..=1531 => 35, 1532..=1606 => 36, 1607..=1681 => 37, 1682..=1756 => 38, 1757..=1831 => 39,
    1832..=1916 => 40, 1917..=2001 => 41, 2002..=2086 => 42, 2087..=2171 => 43, 2172..=2256 => 44,
    2257..=2351 => 45, 2352..=2446 => 46, 2447..=2541 => 47, 2542..=2636 => 48, 2637..=2731 => 49,
    2732..=2839 => 50, 2840..=2959 => 51, 2960..=3079 => 52, 3080..=3199 => 53, 3200..=3319 => 54,
    3320..=3449 => 55, 3450..=3579 => 56, 3580..=3709 => 57, 3710..=3839 => 58, 3840..=3969 => 59,
    3970..=4109 => 60, 4110..=4249 => 61, 4250..=4389 => 62, 4390..=4529 => 63, 4530..=4669 => 64,
    4670..=4819 => 65, 4820..=4969 => 66, 4970..=5119 => 67, 5120..=5269 => 68, 5270..=5419 => 69,
    5420..=5574 => 70, 5575..=5729 => 71, 5730..=5884 => 72, 5885..=6039 => 73, 6040..=6194 => 74,
    6195..=6349 => 75, 6350..=6504 => 76, 6505..=6659 => 77, 6660..=6814 => 78, 6815..=6969 => 79,
    6970..=7124 => 80, 7125..=7279 => 81, 7280..=7434 => 82, 7435..=7589 => 83, 7590..=7744 => 84,
    7745..=7899 => 85, 7900..=8054 => 86, 8055..=8209 => 87, 8210..=8364 => 88, 8365..=8519 => 89,
    8520..=8674 => 90, 8675..=8829 => 91, 8830..=8984 => 92, 8985..=9139 => 93, 9140..=9294 => 94,
    9295..=9449 => 95, 9450..=9604 => 96, 9605..=9759 => 97, 9760..=9914 => 98, 9915..=10069 => 99,
    10070..=10224 => 100, 10225..=10379 => 101, 10380..=10534 => 102, 10535..=10689 => 103, 10690..=10844 => 104,
    10845..=10999 => 105, 11000..=11154 => 106, 11155..=11309 => 107, 11310..=11464 => 108, 11465..=11619 => 109,
    11620..=11774 => 110, 11775..=11929 => 111, 11930..=12084 => 112, 12085..=12239 => 113, 12240..=12394 => 114,
    12395..=12549 => 115, 12550..=12704 => 116, 12705..=12859 => 117, 12860..=13014 => 118, 13015..=13169 => 119,
    13170..=13324 => 120, 13325..=13479 => 121, 13480..=13634 => 122, 13635..=13789 => 123, 13790..=13944 => 124,
    13945..=14099 => 125, 14100..=14254 => 126, 14255..=14409 => 127, 14410..=14564 => 128, 14565..=14719 => 129,
    14720..=14874 => 130, 14875..=15029 => 131, 15030..=15184 => 132, 15185..=15339 => 133, 15340..=15494 => 134,
    15495..=15649 => 135, 15650..=15804 => 136, 15805..=15959 => 137, 15960..=16114 => 138, 16115..=16269 => 139,
    16270..=16424 => 140, 16425..=16579 => 141, 16580..=16734 => 142, 16735..=16889 => 143, 16890..=17044 => 144,
    17045..=17199 => 145, 17200..=17354 => 146, 17355..=17509 => 147, 17510..=17664 => 148, 17665..=17819 => 149,
    17820..=17974 => 150, 17975..=18129 => 151, 18130..=18284 => 152, 18285..=18439 => 153, 18440..=18594 => 154,
    18595..=18749 => 155, 18750..=18904 => 156, 18905..=19059 => 157, 19060..=19214 => 158, 19215..=19369 => 159,
    19370..=19524 => 160, 19525..=19679 => 161, 19680..=19834 => 162, 19835..=19989 => 163, 19990..=20144 => 164,
    20145..=20299 => 165, 20300..=20454 => 166, 20455..=20609 => 167, 20610..=20764 => 168, 20765..=20919 => 169,
    20920..=21074 => 170, 21075..=21229 => 171, 21230..=21384 => 172, 21385..=21539 => 173, 21540..=21694 => 174,
    21695..=21849 => 175, 21850..=22004 => 176, 22005..=22159 => 177, 22160..=22314 => 178, 22315..=22469 => 179,
    22470..=22624 => 180, 22625..=22779 => 181, 22780..=22934 => 182, 22935..=23089 => 183, 23090..=23244 => 184,
    23245..=23399 => 185, 23400..=23554 => 186, 23555..=23709 => 187, 23710..=23864 => 188, 23865..=24019 => 189,
    24020..=24174 => 190, 24175..=24329 => 191, 24330..=24484 => 192, 24485..=24639 => 193, 24640..=24794 => 194,
    24795..=24949 => 195, 24950..=25104 => 196, 25105..=25259 => 197, 25260..=25414 => 198, 25415..=25569 => 199,
    25570..=u32::MAX => 200
  }
}
