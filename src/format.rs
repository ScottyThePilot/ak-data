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
  HandbookInfoTable,
  EquipTable
);

pub(crate) struct DataFiles {
  character_table: CharacterTable,
  character_meta_table: CharacterMetaTable,
  skill_table: SkillTable,
  building_data: BuildingData,
  item_table: ItemTable,
  handbook_info_table: HandbookInfoTable,
  equip_table: EquipTable
}

impl DataFiles {
  pub(crate) async fn from_local(gamedata_dir: &Path) -> Result<Self, crate::Error> {
    tokio::try_join!(
      crate::options::get_data_file_local::<CharacterTable>(gamedata_dir),
      crate::options::get_data_file_local::<CharacterMetaTable>(gamedata_dir),
      crate::options::get_data_file_local::<SkillTable>(gamedata_dir),
      crate::options::get_data_file_local::<BuildingData>(gamedata_dir),
      crate::options::get_data_file_local::<ItemTable>(gamedata_dir),
      crate::options::get_data_file_local::<HandbookInfoTable>(gamedata_dir),
      crate::options::get_data_file_local::<EquipTable>(gamedata_dir)
    ).map(Self::from)
  }

  pub(crate) async fn from_remote(options: &Options) -> Result<Self, crate::Error> {
    tokio::try_join!(
      crate::options::get_data_file_remote::<CharacterTable>(options),
      crate::options::get_data_file_remote::<CharacterMetaTable>(options),
      crate::options::get_data_file_remote::<SkillTable>(options),
      crate::options::get_data_file_remote::<BuildingData>(options),
      crate::options::get_data_file_remote::<ItemTable>(options),
      crate::options::get_data_file_remote::<HandbookInfoTable>(options),
      crate::options::get_data_file_remote::<EquipTable>(options)
    ).map(Self::from)
  }

  pub(crate) fn into_game_data(mut self, update_info: UpdateInfo) -> GameData {
    //let mut handbook = self.handbook_info_table;
    let alters = self.character_meta_table.into_alters();
    let operators = recollect_filter(self.character_table, |(id, character)| {
      let operator = character.into_operator(
        id.clone(),
        &self.building_data,
        &self.skill_table,
        &mut self.handbook_info_table,
        &mut self.equip_table
      );

      operator.map(|operator| (id, operator))
    });

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
  fn from((ct, cmt, st, bd, it, hbit, et): DataFilesTuple) -> Self {
    DataFiles {
      character_table: ct,
      character_meta_table: cmt,
      skill_table: st,
      building_data: bd,
      item_table: it,
      handbook_info_table: hbit,
      equip_table: et
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
  #[serde(rename = "potentialItemId")]
  #[serde(deserialize_with = "deserialize_maybe_empty_str")]
  potential_item_id: Option<String>,
  #[serde(rename = "nationId")]
  nation_id: Option<String>,
  #[serde(rename = "groupId")]
  group_id: Option<String>,
  #[serde(rename = "teamId")]
  team_id: Option<String>,
  #[serde(rename = "displayNumber")]
  display_number: Option<String>,
  #[serde(deserialize_with = "deserialize_maybe_empty_str")]
  appellation: Option<String>,
  position: CharacterTablePosition,
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
  skills: Vec<CharacterTableSkill>,
  #[serde(deserialize_with = "deserialize_or_default")]
  talents: Vec<CharacterTableTalent>,
  #[serde(rename = "potentialRanks")]
  potential_ranks: Vec<CharacterTablePotentialRank>,
  #[serde(rename = "favorKeyFrames")]
  #[serde(deserialize_with = "deserialize_maybe_option_array")]
  favor_key_frames: Option<[CharacterTableKeyFrame; 2]>
}

impl CharacterTableEntry {
  pub(crate) fn into_operator(
    self,
    id: String,
    building_data: &BuildingData,
    skill_table: &SkillTable,
    handbook_info_table: &mut HandbookInfoTable,
    equip_table: &mut EquipTable
  ) -> Option<Operator> {
    if self.is_unobtainable { return None };
    let display_number = self.display_number?;
    let profession = self.profession.into_profession()?;
    let sub_profession = self.sub_profession.into_sub_profession()?;
    let position = self.position.into_position()?;

    let mut promotions = self.phases.into_iter()
      .map(CharacterTablePhase::into_operator_promotion);
    let promotion_none = promotions.next()?;
    let promotion_elite1 = promotions.next();
    let promotion_elite2 = promotions.next();

    let potential = recollect(self.potential_ranks, CharacterTablePotentialRank::into_operator_potential);
    let skills = recollect_maybe(self.skills, |character_table_skill| character_table_skill.into_operator_skill(skill_table))?;
    let talents = recollect_maybe(self.talents, CharacterTableTalent::into_operator_talent)?;
    let modules = equip_table.take_operator_modules(&id).unwrap_or_default();
    let base_skills = building_data.get_operator_base_skill(&id);
    let file = handbook_info_table.take_operator_file(&id)?;

    Some(Operator {
      id,
      name: self.name,
      nation_id: self.nation_id,
      group_id: self.group_id,
      team_id: self.team_id,
      display_number,
      position,
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
      potential_item: self.potential_item_id,
      potential,
      skills,
      talents,
      modules,
      base_skills,
      file
    })
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CharacterTablePhase {
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
  fn into_operator_attributes(self) -> OperatorAttributes {
    OperatorAttributes {
      level: self.level,
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
  cost: u32,
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

#[repr(u8)]
#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) enum CharacterTablePosition {
  #[serde(rename = "MELEE")]
  Melee,
  #[serde(rename = "RANGED")]
  Ranged,
  #[serde(rename = "ALL")]
  All,
  #[serde(rename = "NONE")]
  None
}

impl CharacterTablePosition {
  fn into_position(self) -> Option<Position> {
    match self {
      CharacterTablePosition::Melee => Some(Position::Melee),
      CharacterTablePosition::Ranged => Some(Position::Ranged),
      CharacterTablePosition::All => None,
      CharacterTablePosition::None => None
    }
  }
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
    Some(OperatorTalent {
      phases: recollect_maybe(self.phases, CharacterTableTalentCandidate::into_operator_talent_phase)?
    })
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
    recollect(blackboard, |item| (item.key, item.value))
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
fn deserialize_maybe_empty_str<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
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

fn deserialize_maybe_option_array<'de, D: Deserializer<'de>, const N: usize, T>(deserializer: D) -> Result<Option<[T; N]>, D::Error>
where T: Deserialize<'de> {
  <Option<Vec<T>>>::deserialize(deserializer).map(|v| {
    v.and_then(|v| <[T; N]>::try_from(v).ok())
  })
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
    recollect(item_cost, |item| (item.item_id, item.count))
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
      upgrades: recollect(self.phases, BuildingDataRoomPhase::into_building_upgrade)
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
  category: BuildingDataBuffCategory,
  #[serde(rename = "roomType")]
  room_type: BuildingDataRoomId
}

impl BuildingDataBuff {
  pub(crate) fn to_operator_base_skill_phase(&self, condition: CharCondition) -> OperatorBaseSkillPhase {
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
pub(crate) enum BuildingDataBuffCategory {
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
    recollect(self.items, |(id, item)| (id, item.into_item()))
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
    OperatorFile {
      id: self.char_id,
      illustrator_name: self.illustrator_name,
      entries: recollect(self.story_entries, HandbookStoryEntry::into_operator_file_entry)
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

static RX_TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[@$\w.]+>|</>").unwrap());
static RX_TEMPLATE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{[\w:.%\-@\[\]]+\}").unwrap());

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

#[derive(Debug, Clone, Copy)]
enum FormattingSuffix {
  DecimalPercent, // :0.0%
  IntegerPercent, // :0%
  Decimal, // :0.0
  Integer, // :0
  None
}

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
  fn take_operator_modules(&mut self, id: &str) -> Option<Vec<OperatorModule>> {
    let character_equip_list = self.character_equip_list.remove(id)?;
    recollect_maybe(character_equip_list.iter().skip(1).cloned(), |character_equip_id| {
      self.equip_list.remove(&character_equip_id).and_then(|equip_table_equip| {
        equip_table_equip.into_operator_module(&self.mission_list)
      })
    })
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct EquipTableEquip {
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
pub(crate) struct EquipTableMission {
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
pub(crate) enum EquipTablePhase {
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

fn recollect<T, U, I, C, F>(i: I, f: F) -> C
where I: IntoIterator<Item = T>, C: FromIterator<U>, F: FnMut(T) -> U {
  i.into_iter().map(f).collect()
}

fn recollect_maybe<T, U, I, C, F>(i: I, f: F) -> Option<C>
where I: IntoIterator<Item = T>, C: FromIterator<U>, F: FnMut(T) -> Option<U> {
  recollect(i, f)
}

fn recollect_filter<T, U, I, C, F>(i: I, f: F) -> C
where I: IntoIterator<Item = T>, C: FromIterator<U>, F: FnMut(T) -> Option<U> {
  i.into_iter().filter_map(f).collect()
}
