use once_cell::sync::Lazy;
use regex::{Regex, Captures};
use serde::de::{Deserialize, DeserializeOwned, Deserializer};
use uord::UOrd;

use crate::game_data::*;
use crate::options::Options;

use std::borrow::Cow;
use std::collections::HashMap;
use std::num::NonZeroU8;
use std::path::Path;



pub(crate) type DataFilesTuple = (
  CharacterTable,
  CharacterMetaTable,
  SkillTable,
  BuildingData,
  ItemTable,
  HandbookInfoTable
);

pub(crate) struct DataFiles {
  character_table: CharacterTable,
  character_meta_table: CharacterMetaTable,
  skill_table: SkillTable,
  building_data: BuildingData,
  item_table: ItemTable,
  handbook_info_table: HandbookInfoTable
}

impl DataFiles {
  pub(crate) async fn from_local(gamedata_dir: &Path) -> Result<Self, crate::Error> {
    tokio::try_join!(
      crate::options::get_data_file_local::<CharacterTable>(gamedata_dir),
      crate::options::get_data_file_local::<CharacterMetaTable>(gamedata_dir),
      crate::options::get_data_file_local::<SkillTable>(gamedata_dir),
      crate::options::get_data_file_local::<BuildingData>(gamedata_dir),
      crate::options::get_data_file_local::<ItemTable>(gamedata_dir),
      crate::options::get_data_file_local::<HandbookInfoTable>(gamedata_dir)
    ).map(Self::from)
  }

  pub(crate) async fn from_remote(options: &Options) -> Result<Self, crate::Error> {
    tokio::try_join!(
      crate::options::get_data_file_remote::<CharacterTable>(options),
      crate::options::get_data_file_remote::<CharacterMetaTable>(options),
      crate::options::get_data_file_remote::<SkillTable>(options),
      crate::options::get_data_file_remote::<BuildingData>(options),
      crate::options::get_data_file_remote::<ItemTable>(options),
      crate::options::get_data_file_remote::<HandbookInfoTable>(options)
    ).map(Self::from)
  }

  pub(crate) fn into_game_data(self, update_info: UpdateInfo) -> GameData {
    let mut handbook = self.handbook_info_table;
    let alters = self.character_meta_table.into_alters();
    let operators = self.character_table.into_iter()
      .filter_map(|(id, character)| {
        let operator = character.into_operator(
          id.clone(),
          &self.building_data,
          &self.skill_table,
          &mut handbook
        );

        operator.map(|operator| (id, operator))
      })
      .collect::<HashMap<String, Operator>>();
    let items = self.item_table.into_items();
    let buildings = self.building_data.into_buildings();

    GameData {
      update_info,
      alters,
      operators,
      items,
      buildings
    }
  }
}

impl From<DataFilesTuple> for DataFiles {
  fn from((ct, cmt, st, bd, it, hbit): DataFilesTuple) -> Self {
    DataFiles {
      character_table: ct,
      character_meta_table: cmt,
      skill_table: st,
      building_data: bd,
      item_table: it,
      handbook_info_table: hbit
    }
  }
}



macro_rules! assertion_some {
  ($expr:expr, $literal:literal $(, $($t:tt)*)?) => (match cfg!(feature = "assertions") {
    true => if let Some(value) = $expr { value } else { panic!($literal $(, $($t)*)?) },
    false => $expr?
  });
}

pub(crate) trait DataFile: DeserializeOwned {
  const LOCATION: &'static str;
  const IDENTIFIER: &'static str;
}

impl DataFile for CharacterMetaTable {
  const LOCATION: &'static str = "excel/char_meta_table.json";
  const IDENTIFIER: &'static str = "char_meta_table";
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CharacterMetaTable {
  #[serde(rename = "spCharGroups")]
  sp_char_groups: HashMap<String, Vec<String>>
}

impl CharacterMetaTable {
  pub(crate) fn into_alters(self) -> Vec<UOrd<String>> {
    self.sp_char_groups.into_values()
      .filter_map(|value| <[String; 2]>::try_from(value).ok())
      .map(|[a, b]| UOrd::new(a, b))
      .collect()
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CharacterTableEntry {
  name: String,
  #[serde(rename = "nationId")]
  nation_id: Option<String>,
  #[serde(rename = "groupId")]
  group_id: Option<String>,
  #[serde(rename = "teamId")]
  team_id: Option<String>,
  #[serde(rename = "displayNumber")]
  display_number: Option<String>,
  #[serde(deserialize_with = "deserialize_appellation")]
  appellation: Option<String>,
  #[serde(rename = "tagList")]
  #[serde(deserialize_with = "deserialize_or_default")]
  recruitment_tags: Vec<String>,
  #[serde(rename = "isNotObtainable")]
  is_unobtainable: bool,
  // omitted fields: isSpChar
  rarity: u8,
  profession: CharacterTableProfession,
  #[serde(rename = "subProfessionId")]
  sub_profession: CharacterTableSubProfession,
  phases: Vec<CharacterTablePhase>,
  #[serde(rename = "favorKeyFrames")]
  #[serde(deserialize_with = "deserialize_or_default")]
  module_phases: Vec<CharacterTableKeyFrame>,
  skills: Vec<CharacterTableSkill>,
  #[serde(deserialize_with = "deserialize_or_default")]
  talents: Vec<CharacterTableTalent>,
  #[serde(rename = "potentialRanks")]
  potential_ranks: Vec<CharacterTablePotentialRank>
}

impl CharacterTableEntry {
  pub(crate) fn into_operator(
    self,
    id: String,
    building_data: &BuildingData,
    skill_table: &SkillTable,
    handbook: &mut HandbookInfoTable
  ) -> Option<Operator> {
    if self.is_unobtainable { return None };
    let display_number = self.display_number?;
    let profession = self.profession.into_profession()?;
    let sub_profession = self.sub_profession.into_sub_profession()?;

    let mut promotions = self.phases.into_iter()
      .map(CharacterTablePhase::into_operator_promotion);
    let promotion_none = promotions.next()?;
    let promotion_elite1 = promotions.next();
    let promotion_elite2 = promotions.next();

    // skip the first module entry because it's always the default module/badge
    let modules = self.module_phases.into_iter().skip(1)
      .map(CharacterTableKeyFrame::into_operator_module)
      .collect();
    let potential = self.potential_ranks.into_iter()
      .map(CharacterTablePotentialRank::into_operator_potential)
      .collect();
    let skills = self.skills.into_iter()
      .map(|character_table_skill| character_table_skill.into_operator_skill(skill_table))
      .collect::<Option<_>>()?;
    let talents = self.talents.into_iter()
      .map(CharacterTableTalent::into_operator_talent)
      .collect::<Option<_>>()?;
    let base_skills = building_data.get_operator_base_skill(&id);
    let operator_file = handbook.take_operator_file(&id);

    Some(Operator {
      id,
      name: self.name,
      nation_id: self.nation_id,
      group_id: self.group_id,
      team_id: self.team_id,
      display_number,
      appellation: self.appellation,
      recruitment_tags: self.recruitment_tags,
      rarity: NonZeroU8::new(self.rarity + 1).unwrap(),
      profession,
      sub_profession,
      promotions: OperatorPromotions {
        none: promotion_none,
        elite1: promotion_elite1,
        elite2: promotion_elite2
      },
      modules,
      potential,
      skills,
      talents,
      base_skills,
      operator_file
    })
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CharacterTablePhase {
  #[serde(rename = "characterPrefabKey")]
  character_prefab_key: String,
  #[serde(rename = "rangeId")]
  range_id: Option<String>,
  #[serde(rename = "maxLevel")]
  max_level: u32,
  #[serde(rename = "attributesKeyFrames")]
  attributes_key_frames: [CharacterTableKeyFrame; 2],
  #[serde(rename = "evolveCost")]
  #[serde(deserialize_with = "deserialize_or_default")]
  upgrade_cost: Vec<ItemCost>
}

impl CharacterTablePhase {
  fn into_operator_promotion(self) -> OperatorPromotion {
    let [min_attributes, max_attributes] = self.attributes_key_frames;
    OperatorPromotion {
      operator_id: self.character_prefab_key,
      attack_range_id: self.range_id,
      min_attributes: min_attributes.into_operator_attributes(),
      max_attributes: max_attributes.into_operator_attributes(),
      max_level: self.max_level,
      upgrade_cost: ItemCost::convert(self.upgrade_cost)
    }
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CharacterTableKeyFrame {
  level: u32,
  data: CharacterTableKeyFrameData
}

impl CharacterTableKeyFrame {
  fn into_operator_module(self) -> OperatorModule {
    OperatorModule {
      attributes: self.into_operator_attributes()
    }
  }

  fn into_operator_attributes(self) -> OperatorAttributes {
    OperatorAttributes {
      level_requirement: self.level,
      max_hp: self.data.max_hp,
      atk: self.data.atk,
      def: self.data.def,
      magic_resistance: self.data.magic_resistance,
      deployment_cost: self.data.cost,
      block_count: self.data.block_count,
      move_speed: self.data.move_speed,
      attack_speed: self.data.attack_speed,
      base_attack_time: self.data.base_attack_time,
      redeploy_time: self.data.respawn_time,
      hp_recovery_per_sec: self.data.hp_recovery_per_sec,
      sp_recovery_per_sec: self.data.sp_recovery_per_sec,
      max_deploy_count: self.data.max_deploy_count,
      max_deck_stack_count: self.data.max_deck_stack_count,
      taunt_level: self.data.taunt_level,
      is_stun_immune: self.data.is_stun_immune,
      is_silence_immune: self.data.is_silence_immune,
      is_sleep_immune: self.data.is_sleep_immune,
      is_frozen_immune: self.data.is_frozen_immune
    }
  }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct CharacterTableKeyFrameData {
  #[serde(rename = "maxHp")]
  max_hp: u32,
  atk: u32,
  def: u32,
  #[serde(rename = "magicResistance")]
  magic_resistance: f32,
  cost: u8,
  #[serde(rename = "blockCnt")]
  block_count: u8,
  #[serde(rename = "moveSpeed")]
  move_speed: f32,
  #[serde(rename = "attackSpeed")]
  attack_speed: f32,
  #[serde(rename = "baseAttackTime")]
  base_attack_time: f32,
  #[serde(rename = "respawnTime")]
  respawn_time: u32,
  #[serde(rename = "hpRecoveryPerSec")]
  hp_recovery_per_sec: f32,
  #[serde(rename = "spRecoveryPerSec")]
  sp_recovery_per_sec: f32,
  #[serde(rename = "maxDeployCount")]
  max_deploy_count: u32,
  #[serde(rename = "maxDeckStackCnt")]
  max_deck_stack_count: u32,
  #[serde(rename = "tauntLevel")]
  taunt_level: i8,
  // omitted fields: massLevel, baseForceLevel
  #[serde(rename = "stunImmune")]
  is_stun_immune: bool,
  #[serde(rename = "silenceImmune")]
  is_silence_immune: bool,
  #[serde(rename = "sleepImmune")]
  is_sleep_immune: bool,
  #[serde(rename = "frozenImmune")]
  is_frozen_immune: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CharacterTableSkill {
  #[serde(rename = "skillId")]
  id: Option<String>,
  #[serde(rename = "overridePrefabKey")]
  override_prefab_key: Option<String>,
  // omitted fields: overrideTokenKey
  #[serde(rename = "levelUpCostCond")]
  #[serde(deserialize_with = "deserialize_option_array")]
  mastery_upgrades: Option<[CharacterTableSkillMastery; 3]>,
  #[serde(rename = "unlockCond")]
  unlock_condition: CharCondition
}

impl CharacterTableSkill {
  fn into_operator_skill(self, skill_table: &SkillTable) -> Option<OperatorSkill> {
    let id = self.id?;
    let skill_table_entry = assertion_some!(skill_table.get(&id), "key {} not present", id);
    let (name, activation, recovery) = assertion_some!(skill_table_entry.name_activation_recovery(), "invalid skill levels");
    let (skill_table_levels7, skill_table_levels3) = skill_table_entry.split_levels()?;
    let levels = skill_table_levels7.clone().map(SkillTableLevel::into_skill_level);
    let mastery = self.mastery_upgrades.zip(skill_table_levels3).map(|(mastery_upgrades, skill_table_levels)| {
      zip_map(mastery_upgrades, skill_table_levels.clone(), CharacterTableSkillMastery::into_operator_skill_mastery)
    });

    Some(OperatorSkill {
      id,
      name,
      prefab_key: self.override_prefab_key,
      condition: self.unlock_condition.into_promotion_and_level(),
      activation,
      recovery,
      levels,
      mastery
    })
  }
}

// array::zip is not stabilized :(
fn zip_map<T, U, V, F, const N: usize>(array_t: [T; N], array_u: [U; N], mut f: F) -> [V; N]
where F: FnMut(T, U) -> V {
  Iterator::zip(array_t.into_iter(), array_u.into_iter())
    .map(|(t, u)| f(t, u))
    .collect::<Vec<V>>()
    .try_into().ok()
    .unwrap()
}

fn all_equal<T, I>(mut iter: I) -> Option<T>
where T: PartialEq, I: Iterator<Item = T> {
  let item_first = iter.next()?;
  for item in iter {
    if item != item_first {
      return None;
    };
  };

  Some(item_first)
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CharacterTableSkillMastery {
  #[serde(rename = "unlockCond")]
  unlock_condition: CharCondition,
  #[serde(rename = "lvlUpTime")]
  level_up_time: u32,
  #[serde(rename = "levelUpCost")]
  #[serde(deserialize_with = "deserialize_or_default")]
  level_up_cost: Vec<ItemCost>
}

impl CharacterTableSkillMastery {
  fn into_operator_skill_mastery(self, skill_table_level: SkillTableLevel) -> OperatorSkillMastery {
    OperatorSkillMastery {
      condition: self.unlock_condition.into_promotion_and_level(),
      upgrade_time: self.level_up_time,
      upgrade_cost: ItemCost::convert(self.level_up_cost),
      level: skill_table_level.into_skill_level()
    }
  }
}

#[repr(transparent)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CharacterTableTalent {
  #[serde(rename = "candidates")]
  #[serde(deserialize_with = "deserialize_or_default")]
  phases: Vec<CharacterTableTalentCandidate>
}

impl CharacterTableTalent {
  fn into_operator_talent(self) -> Option<OperatorTalent> {
    let phases: Vec<OperatorTalentPhase> = self.phases.into_iter()
      .map(CharacterTableTalentCandidate::into_operator_talent_phase)
      .collect::<Option<_>>()?;
    Some(OperatorTalent { phases })
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CharacterTableTalentCandidate {
  #[serde(rename = "unlockCondition")]
  unlock_condition: CharCondition,
  #[serde(rename = "requiredPotentialRank")]
  required_potential_rank: u8,
  #[serde(rename = "prefabKey")]
  prefab_key: String,
  name: Option<String>,
  description: Option<String>,
  #[serde(rename = "rangeId")]
  range_id: Option<String>,
  blackboard: Vec<CharacterTableTalentBlackboard>
}

impl CharacterTableTalentCandidate {
  fn into_operator_talent_phase(self) -> Option<OperatorTalentPhase> {
    Some(OperatorTalentPhase {
      name: self.name?,
      description: strip_tags(&self.description?).into_owned(),
      condition: self.unlock_condition.into_promotion_and_level(),
      required_potential: self.required_potential_rank,
      prefab_key: self.prefab_key,
      attack_range_id: self.range_id,
      effects: CharacterTableTalentBlackboard::convert(self.blackboard)
    })
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CharacterTableTalentBlackboard {
  key: String,
  value: f32
}

impl CharacterTableTalentBlackboard {
  fn convert(blackboard: Vec<Self>) -> HashMap<String, f32> {
    blackboard.into_iter().map(|item| (item.key, item.value)).collect()
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CharacterTablePotentialRank {
  #[serde(rename = "type")]
  potential_type: u32,
  description: String
}

impl CharacterTablePotentialRank {
  fn into_operator_potential(self) -> OperatorPotential {
    let CharacterTablePotentialRank { potential_type, description } = self;
    let description = strip_tags(&description).into_owned();
    OperatorPotential { potential_type, description }
  }
}

#[inline]
fn deserialize_appellation<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
  let value = String::deserialize(deserializer)?;
  Ok(if value.trim().is_empty() { None } else { Some(value) })
}

#[inline]
fn deserialize_or_default<'de, D: Deserializer<'de>, T>(deserializer: D) -> Result<T, D::Error>
where T: Deserialize<'de> + Default {
  <Option<T>>::deserialize(deserializer).map(Option::unwrap_or_default)
}

#[inline]
fn deserialize_option_array<'de, D: Deserializer<'de>, const N: usize, T>(deserializer: D) -> Result<Option<[T; N]>, D::Error>
where T: Deserialize<'de> {
  <Vec<T>>::deserialize(deserializer).map(|v| <[T; N]>::try_from(v).ok())
}

#[inline]
fn deserialize_negative_int<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u32>, D::Error> {
  let value = i32::deserialize(deserializer)?;
  Ok(if value.is_negative() { None } else { Some(value as u32) })
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ItemCost {
  #[serde(rename = "id")]
  item_id: String,
  count: u32
}

impl ItemCost {
  fn convert(item_cost: Vec<Self>) -> HashMap<String, u32> {
    item_cost.into_iter().map(|item| (item.item_id, item.count)).collect()
  }
}

impl DataFile for CharacterTable {
  const LOCATION: &'static str = "excel/character_table.json";
  const IDENTIFIER: &'static str = "character_table";
}

pub(crate) type CharacterTable = HashMap<String, CharacterTableEntry>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum CharacterTableProfession {
  #[serde(rename = "CASTER")]
  Caster,
  #[serde(rename = "MEDIC")]
  Medic,
  #[serde(rename = "PIONEER")]
  Vanguard,
  #[serde(rename = "SNIPER")]
  Sniper,
  #[serde(rename = "SPECIAL")]
  Specialist,
  #[serde(rename = "SUPPORT")]
  Support,
  #[serde(rename = "TANK")]
  Tank,
  #[serde(rename = "WARRIOR")]
  Guard,
  #[serde(rename = "TOKEN")]
  Token,
  #[serde(rename = "TRAP")]
  Trap
}

impl CharacterTableProfession {
  pub(crate) fn into_profession(self) -> Option<Profession> {
    match self {
      CharacterTableProfession::Caster => Some(Profession::Caster),
      CharacterTableProfession::Medic => Some(Profession::Medic),
      CharacterTableProfession::Vanguard => Some(Profession::Vanguard),
      CharacterTableProfession::Sniper => Some(Profession::Sniper),
      CharacterTableProfession::Specialist => Some(Profession::Specialist),
      CharacterTableProfession::Support => Some(Profession::Support),
      CharacterTableProfession::Tank => Some(Profession::Tank),
      CharacterTableProfession::Guard => Some(Profession::Guard),
      _ => None
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum CharacterTableSubProfession {
  // Casters
  #[serde(rename = "blastcaster")]
  BlastCaster,
  #[serde(rename = "chain")]
  ChainCaster,
  #[serde(rename = "corecaster")]
  CoreCaster,
  #[serde(rename = "funnel")]
  MechAccordCaster,
  #[serde(rename = "mystic")]
  MysticCaster,
  #[serde(rename = "phalanx")]
  PhalanxCaster,
  #[serde(rename = "splashcaster")]
  SplashCaster,
  // Medics
  #[serde(rename = "healer")]
  Therapist,
  #[serde(rename = "physician")]
  Medic,
  #[serde(rename = "ringhealer")]
  MultiTargetMedic,
  #[serde(rename = "wandermedic")]
  WanderingMedic,
  // Vanguards
  #[serde(rename = "bearer")]
  StandardBearer,
  #[serde(rename = "charger")]
  Charger,
  #[serde(rename = "pioneer")]
  Pioneer,
  #[serde(rename = "tactician")]
  Tactician,
  // Snipers
  #[serde(rename = "aoesniper")]
  Artilleryman,
  #[serde(rename = "bombarder")]
  Flinger,
  #[serde(rename = "closerange")]
  Heavyshooter,
  #[serde(rename = "fastshot")]
  Marksman,
  #[serde(rename = "longrange")]
  Deadeye,
  #[serde(rename = "reaperrange")]
  Spreadshooter,
  #[serde(rename = "siegesniper")]
  Besieger,
  // Specialists
  #[serde(rename = "dollkeeper")]
  Dollkeeper,
  #[serde(rename = "executor")]
  Executor,
  #[serde(rename = "geek")]
  Geek,
  #[serde(rename = "hookmaster")]
  Hookmaster,
  #[serde(rename = "merchant")]
  Merchant,
  #[serde(rename = "pusher")]
  PushStroker,
  #[serde(rename = "stalker")]
  Ambusher,
  #[serde(rename = "traper")]
  Trapmaster,
  // Supports
  #[serde(rename = "bard")]
  Bard,
  #[serde(rename = "blessing")]
  Abjurer,
  #[serde(rename = "craftsman")]
  Artificer,
  #[serde(rename = "slower")]
  DecelBinder,
  #[serde(rename = "summoner")]
  Summoner,
  #[serde(rename = "underminer")]
  Hexer,
  // Tanks
  #[serde(rename = "artsprotector")]
  ArtsProtector,
  #[serde(rename = "duelist")]
  Duelist,
  #[serde(rename = "fortress")]
  Fortress,
  #[serde(rename = "guardian")]
  Guardian,
  #[serde(rename = "protector")]
  Protector,
  #[serde(rename = "unyield")]
  Juggernaut,
  // Guards
  #[serde(rename = "artsfghter")]
  ArtsFighter,
  #[serde(rename = "centurion")]
  Centurion,
  #[serde(rename = "fearless")]
  Dreadnought,
  #[serde(rename = "fighter")]
  Fighter,
  #[serde(rename = "instructor")]
  Instructor,
  #[serde(rename = "librator")]
  Liberator,
  #[serde(rename = "lord")]
  Lord,
  #[serde(rename = "musha")]
  Musha,
  #[serde(rename = "reaper")]
  Reaper,
  #[serde(rename = "sword")]
  Swordmaster,
  // Other
  #[serde(rename = "none1")]
  None1,
  #[serde(rename = "none2")]
  None2,
  #[serde(rename = "notchar1")]
  NotChar1,
  #[serde(rename = "notchar2")]
  NotChar2
}

impl CharacterTableSubProfession {
  pub(crate) fn into_sub_profession(self) -> Option<SubProfession> {
    match self {
      CharacterTableSubProfession::BlastCaster => Some(SubProfession::BlastCaster),
      CharacterTableSubProfession::ChainCaster => Some(SubProfession::ChainCaster),
      CharacterTableSubProfession::CoreCaster => Some(SubProfession::CoreCaster),
      CharacterTableSubProfession::MechAccordCaster => Some(SubProfession::MechAccordCaster),
      CharacterTableSubProfession::MysticCaster => Some(SubProfession::MysticCaster),
      CharacterTableSubProfession::PhalanxCaster => Some(SubProfession::PhalanxCaster),
      CharacterTableSubProfession::SplashCaster => Some(SubProfession::SplashCaster),
      CharacterTableSubProfession::Therapist => Some(SubProfession::Therapist),
      CharacterTableSubProfession::Medic => Some(SubProfession::Medic),
      CharacterTableSubProfession::MultiTargetMedic => Some(SubProfession::MultiTargetMedic),
      CharacterTableSubProfession::WanderingMedic => Some(SubProfession::WanderingMedic),
      CharacterTableSubProfession::StandardBearer => Some(SubProfession::StandardBearer),
      CharacterTableSubProfession::Charger => Some(SubProfession::Charger),
      CharacterTableSubProfession::Pioneer => Some(SubProfession::Pioneer),
      CharacterTableSubProfession::Tactician => Some(SubProfession::Tactician),
      CharacterTableSubProfession::Artilleryman => Some(SubProfession::Artilleryman),
      CharacterTableSubProfession::Flinger => Some(SubProfession::Flinger),
      CharacterTableSubProfession::Heavyshooter => Some(SubProfession::Heavyshooter),
      CharacterTableSubProfession::Marksman => Some(SubProfession::Marksman),
      CharacterTableSubProfession::Deadeye => Some(SubProfession::Deadeye),
      CharacterTableSubProfession::Spreadshooter => Some(SubProfession::Spreadshooter),
      CharacterTableSubProfession::Besieger => Some(SubProfession::Besieger),
      CharacterTableSubProfession::Dollkeeper => Some(SubProfession::Dollkeeper),
      CharacterTableSubProfession::Executor => Some(SubProfession::Executor),
      CharacterTableSubProfession::Geek => Some(SubProfession::Geek),
      CharacterTableSubProfession::Hookmaster => Some(SubProfession::Hookmaster),
      CharacterTableSubProfession::Merchant => Some(SubProfession::Merchant),
      CharacterTableSubProfession::PushStroker => Some(SubProfession::PushStroker),
      CharacterTableSubProfession::Ambusher => Some(SubProfession::Ambusher),
      CharacterTableSubProfession::Trapmaster => Some(SubProfession::Trapmaster),
      CharacterTableSubProfession::Bard => Some(SubProfession::Bard),
      CharacterTableSubProfession::Abjurer => Some(SubProfession::Abjurer),
      CharacterTableSubProfession::Artificer => Some(SubProfession::Artificer),
      CharacterTableSubProfession::DecelBinder => Some(SubProfession::DecelBinder),
      CharacterTableSubProfession::Summoner => Some(SubProfession::Summoner),
      CharacterTableSubProfession::Hexer => Some(SubProfession::Hexer),
      CharacterTableSubProfession::ArtsProtector => Some(SubProfession::ArtsProtector),
      CharacterTableSubProfession::Duelist => Some(SubProfession::Duelist),
      CharacterTableSubProfession::Fortress => Some(SubProfession::Fortress),
      CharacterTableSubProfession::Guardian => Some(SubProfession::Guardian),
      CharacterTableSubProfession::Protector => Some(SubProfession::Protector),
      CharacterTableSubProfession::Juggernaut => Some(SubProfession::Juggernaut),
      CharacterTableSubProfession::ArtsFighter => Some(SubProfession::ArtsFighter),
      CharacterTableSubProfession::Centurion => Some(SubProfession::Centurion),
      CharacterTableSubProfession::Dreadnought => Some(SubProfession::Dreadnought),
      CharacterTableSubProfession::Fighter => Some(SubProfession::Fighter),
      CharacterTableSubProfession::Instructor => Some(SubProfession::Instructor),
      CharacterTableSubProfession::Liberator => Some(SubProfession::Liberator),
      CharacterTableSubProfession::Lord => Some(SubProfession::Lord),
      CharacterTableSubProfession::Musha => Some(SubProfession::Musha),
      CharacterTableSubProfession::Reaper => Some(SubProfession::Reaper),
      CharacterTableSubProfession::Swordmaster => Some(SubProfession::Swordmaster),
      _ => None
    }
  }
}

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
  fn into_buildings(self) -> HashMap<BuildingType, Building> {
    self.rooms.into_values()
      .map(|building_data_room| {
        (building_data_room.id.into_building_type(), building_data_room.into_building())
      })
      .collect()
  }

  fn get_operator_base_skill(&self, id: &str) -> Vec<OperatorBaseSkill> {
    // if an operator can't be found, just return an empty array of base skills
    self.chars.get(id).map_or_else(Vec::new, |BuildingDataChar { buffs, .. }| {
      buffs.iter().filter_map(|buff| buff.to_operator_base_skill(self)).collect()
    })
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BuildingDataRoom {
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
      upgrades: self.phases.into_iter()
        .map(BuildingDataRoomPhase::into_building_upgrade)
        .collect()
    }
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BuildingDataRoomPhase {
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
pub(crate) struct BuildingDataBuildCost {
  items: Vec<ItemCost>,
  labor: u32
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) struct BuildingDataRoomSize {
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
pub(crate) enum BuildingDataRoomId {
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
pub(crate) struct BuildingDataChar {
  // omitted fields: charId
  #[serde(rename = "buffChar")]
  buffs: Vec<BuildingDataCharBuff>
}

#[repr(transparent)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BuildingDataCharBuff {
  #[serde(rename = "buffData")]
  phases: Vec<BuildingDataCharBuffPhase>
}

impl BuildingDataCharBuff {
  pub(crate) fn to_operator_base_skill(&self, building_data: &BuildingData) -> Option<OperatorBaseSkill> {
    if self.phases.is_empty() { return None };
    let phases = self.phases.iter()
      .map(|phase| phase.to_operator_base_skill_phase(building_data))
      .collect();
    Some(OperatorBaseSkill { phases })
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BuildingDataCharBuffPhase {
  #[serde(rename = "buffId")]
  id: String,
  #[serde(rename = "cond")]
  condition: CharCondition
}

impl BuildingDataCharBuffPhase {
  pub(crate) fn to_operator_base_skill_phase(&self, building_data: &BuildingData) -> OperatorBaseSkillPhase {
    let BuildingDataCharBuffPhase { id, condition } = self;
    building_data.buffs[id].to_operator_base_skill_phase(condition.clone())
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CharCondition {
  phase: u32,
  level: u32
}

impl CharCondition {
  pub(crate) fn into_promotion_and_level(self) -> PromotionAndLevel {
    let CharCondition { phase, level } = self;
    let promotion = match phase {
      0 => Promotion::None,
      1 => Promotion::Elite1,
      2 => Promotion::Elite2,
      p => panic!("invalid promotion {p:?}")
    };

    PromotionAndLevel { promotion, level }
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BuildingDataBuff {
  // omitted fields: buffId
  #[serde(rename = "buffName")]
  name: String,
  #[serde(rename = "sortId")]
  sort: u32,
  #[serde(rename = "buffCategory")]
  category: String,
  #[serde(rename = "roomType")]
  room_type: String
}

impl BuildingDataBuff {
  pub(crate) fn to_operator_base_skill_phase(&self, condition: CharCondition) -> OperatorBaseSkillPhase {
    let condition = condition.into_promotion_and_level();
    let BuildingDataBuff { name, sort, category, room_type, .. } = self.clone();
    OperatorBaseSkillPhase { name, condition, sort, category, room_type }
  }
}

impl DataFile for ItemTable {
  const LOCATION: &'static str = "excel/item_table.json";
  const IDENTIFIER: &'static str = "item_table";
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ItemTable {
  items: HashMap<String, ItemTableItem>
}

impl ItemTable {
  pub(crate) fn into_items(self) -> HashMap<String, Item> {
    self.items.into_iter()
      .map(|(id, item)| (id, item.into_item()))
      .collect()
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ItemTableItem {
  #[serde(rename = "itemId")]
  id: String,
  name: String,
  description: Option<String>,
  rarity: u32,
  usage: Option<String>,
  #[serde(rename = "obtainApproach")]
  obtain: Option<String>,
  #[serde(rename = "classifyType")]
  classify: ItemTableItemClassify,
  #[serde(rename = "itemType")]
  item_type: String
}

impl ItemTableItem {
  pub(crate) fn into_item(self) -> Item {
    Item {
      id: self.id,
      name: self.name,
      description: self.description,
      rarity: self.rarity,
      usage: self.usage,
      obtain: self.obtain,
      item_class: self.classify.into_item_class(),
      item_type: self.item_type
    }
  }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) enum ItemTableItemClassify {
  #[serde(rename = "CONSUME")]
  Consume,
  #[serde(rename = "MATERIAL")]
  Material,
  #[serde(rename = "NONE")]
  None,
  #[serde(rename = "NORMAL")]
  Normal
}

impl ItemTableItemClassify {
  pub(crate) fn into_item_class(self) -> ItemClass {
    match self {
      ItemTableItemClassify::Consume => ItemClass::Consumable,
      ItemTableItemClassify::Normal => ItemClass::BasicItem,
      ItemTableItemClassify::Material => ItemClass::Material,
      ItemTableItemClassify::None => ItemClass::Other
    }
  }
}



impl DataFile for HandbookInfoTable {
  const LOCATION: &'static str = "excel/handbook_info_table.json";
  const IDENTIFIER: &'static str = "handbook_info_table";
}

#[repr(transparent)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct HandbookInfoTable {
  #[serde(rename = "handbookDict")]
  handbook_dict: HashMap<String, HandbookInfoTableEntry>
}

impl HandbookInfoTable {
  fn take_operator_file(&mut self, id: &str) -> Option<OperatorFile> {
    self.handbook_dict.remove(id).map(HandbookInfoTableEntry::into_operator_file)
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct HandbookInfoTableEntry {
  #[serde(rename = "charID")]
  char_id: String,
  #[serde(rename = "drawName")]
  illustrator_name: String,
  #[serde(rename = "storyTextAudio")]
  story_entries: Vec<HandbookStoryEntry>
}

impl HandbookInfoTableEntry {
  fn into_operator_file(self) -> OperatorFile {
    let file_entries = self.story_entries.into_iter()
      .map(HandbookStoryEntry::into_operator_file_entry)
      .collect();

    OperatorFile {
      id: self.char_id,
      illustrator_name: self.illustrator_name,
      entries: file_entries
    }
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct HandbookStoryEntry {
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
pub(crate) struct HandbookStory {
  #[serde(rename = "storyText")]
  story_text: String,
  #[serde(rename = "unLockType")]
  unlock_type: u32,
  #[serde(rename = "unLockParam")]
  unlock_param: HandbookStoryUnlockParam
}

#[derive(Debug, Clone)]
pub(crate) enum HandbookStoryUnlockParam {
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
    fn try_parse_char_condition(v: &str) -> Option<(u32, u32)> {
      v.split_once(';').and_then(|(phase, level)| {
        Some((phase.parse().ok()?, level.parse().ok()?))
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
  fn split_levels(&self) -> Option<(&[SkillTableLevel; 7], Option<&[SkillTableLevel; 3]>)> {
    if self.levels.len() < 7 { return None };
    let (start, end) = self.levels.split_at(7);
    let start: &[SkillTableLevel; 7] = start.try_into().ok()?;
    let end: Option<&[SkillTableLevel; 3]> = end.try_into().ok();
    Some((start, end))
  }

  fn name_activation_recovery(&self) -> Option<(String, SkillActivation, SkillRecovery)> {
    all_equal(self.levels.iter().map(|level| {
      let activation = level.skill_type.into_activation();
      let recovery = level.sp_data.sp_type.into_recovery();
      (level.name.clone(), activation, recovery)
    }))
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SkillTableLevel {
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
  fn into_skill_level(self) -> OperatorSkillLevel {
    let description = self.apply_blackboard();

    OperatorSkillLevel {
      description,
      range_id: self.range_id,
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
pub(crate) struct SkillTableSpData {
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
pub(crate) struct SkillTableBlackboardEntry {
  key: String,
  value: f32
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum SkillTableSpType {
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

impl<'de> Deserialize<'de> for SkillTableSpType {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    struct SkillTableSpTypeVisitor;

    impl<'de> serde::de::Visitor<'de> for SkillTableSpTypeVisitor {
      type Value = SkillTableSpType;

      #[inline]
      fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a positive integer, one of 1, 2, 4 or 8")
      }

      fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
      where E: serde::de::Error {
        match v {
          1 => Ok(SkillTableSpType::AutoRecovery),
          2 => Ok(SkillTableSpType::OffensiveRecovery),
          4 => Ok(SkillTableSpType::DefensiveRecovery),
          8 => Ok(SkillTableSpType::Passive),
          _ => Err(E::invalid_value(serde::de::Unexpected::Unsigned(v), &Self))
        }
      }
    }

    deserializer.deserialize_u64(SkillTableSpTypeVisitor)
  }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum SkillTableSkillType {
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

impl<'de> Deserialize<'de> for SkillTableSkillType {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    struct SkillTypeVisitor;

    impl<'de> serde::de::Visitor<'de> for SkillTypeVisitor {
      type Value = SkillTableSkillType;

      #[inline]
      fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a positive integer, one of 0, 1, or 2")
      }

      fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
      where E: serde::de::Error {
        match v {
          0 => Ok(SkillTableSkillType::Passive),
          1 => Ok(SkillTableSkillType::Manual),
          2 => Ok(SkillTableSkillType::Auto),
          _ => Err(E::invalid_value(serde::de::Unexpected::Unsigned(v), &Self))
        }
      }
    }

    deserializer.deserialize_u64(SkillTypeVisitor)
  }
}

// spType 1 -> auto recovery
// spType 2 -> offensive recovery
// spType 4 -> defensive recovery
// spType 8 -> passive
// skillType 0 -> passive
// skillType 1 -> manual
// skillType 2 -> auto

static RX_TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[@$\w.]+>|</>").unwrap());
static RX_TEMPLATE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{[\w:.%\-@\[\]]+\}").unwrap());
//static RX_TEMPLATE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{[^\n\s>}]+\}").unwrap());

fn strip_tags<'a>(text: &'a str) -> Cow<'a, str> {
  RX_TAG.replace_all(&text, "")
}

fn apply_templates(text: &str, blackboard: HashMap<String, f32>) -> String {
  let text = strip_tags(text);
  let text = RX_TEMPLATE.replace_all(&text, |captures: &Captures| -> String {
    let key = captures.get(0).unwrap().as_str();
    let key = key.trim_matches(&['{', '}'] as &[char]);
    let (key, negative, suffix) = strip_formatting_markers(key);

    if let Some(&blackboard_entry) = blackboard.get(&key) {
      apply_formatting(blackboard_entry, negative, suffix)
    } else {
      if cfg!(feature = "assertions") {
        panic!("assertion failed: unknown key {key:?} encountered");
      } else {
        key.to_uppercase()
      }
    }
  });

  text.into_owned()
}

fn strip_formatting_markers(string: &str) -> (String, bool, FormattingSuffix) {
  let (negative, string) = match string.strip_prefix("-") {
    Some(string) => (true, string),
    None => (false, string)
  };

  if let Some(string) = string.strip_suffix(":0.0%") {
    (string.to_lowercase(), negative, FormattingSuffix::DecimalPercent)
  } else if let Some(string) = string.strip_suffix(":0%") {
    (string.to_lowercase(), negative, FormattingSuffix::IntegerPercent)
  } else if let Some(string) = string.strip_suffix(":0.0") {
    (string.to_lowercase(), negative, FormattingSuffix::Decimal)
  } else if let Some(string) = string.strip_suffix(":0") {
    (string.to_lowercase(), negative, FormattingSuffix::Integer)
  } else {
    (string.to_lowercase(), negative, FormattingSuffix::None)
  }
}

fn apply_formatting(value: f32, negative: bool, suffix: FormattingSuffix) -> String {
  fn r(mut string: String) -> String {
    if string.ends_with("0") { string.pop(); };
    if string.ends_with("0") { string.pop(); };
    if string.ends_with(".") { string.pop(); };
    string
  }

  let value = if negative { -value } else { value };
  let out = match suffix {
    FormattingSuffix::DecimalPercent => r(format!("{:.2}%", value * 100.0)),
    FormattingSuffix::IntegerPercent => format!("{:.0}%", value * 100.0),
    FormattingSuffix::Decimal => r(format!("{value:.2}")),
    FormattingSuffix::Integer => format!("{value:.0}"),
    FormattingSuffix::None => format!("{value}")
  };

  out
}

enum FormattingSuffix {
  DecimalPercent, // :0.0%
  IntegerPercent, // :0%
  Decimal, // :0.0
  Integer, // :0
  None
}
