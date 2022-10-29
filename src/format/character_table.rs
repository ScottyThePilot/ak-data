use crate::format::*;
use crate::format::skill_table::SkillTableLevel;
use crate::game_data::*;

use std::collections::HashMap;
use std::num::NonZeroU8;

impl DataFile for CharacterTable {
  const LOCATION: &'static str = "excel/character_table.json";
  const IDENTIFIER: &'static str = "character_table";
}

pub(crate) type CharacterTable = HashMap<String, CharacterTableEntry>;

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
  pub(super) fn into_operator(
    self,
    id: String,
    building_data: &super::BuildingData,
    skill_table: &super::SkillTable,
    handbook_info_table: &mut super::HandbookInfoTable,
    equip_table: &mut super::EquipTable
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
      potential_item_id: self.potential_item_id,
      potential,
      skills,
      talents,
      modules,
      base_skills,
      trust_bonus: match self.favor_key_frames {
        Some([_, keyframe]) => keyframe.into_operator_trust_attributes(),
        None => OperatorTrustAttributes::default()
      },
      file
    })
  }
}

#[derive(Debug, Clone, Deserialize)]
struct CharacterTablePhase {
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
      min_attributes: min_attributes.into_operator_promotion_attributes(),
      max_attributes: max_attributes.into_operator_promotion_attributes(),
      max_level: self.max_level,
      upgrade_cost: ItemCost::convert(self.upgrade_cost)
    }
  }
}

#[derive(Debug, Clone, Deserialize)]
struct CharacterTableKeyFrame {
  level: u32,
  data: CharacterTableKeyFrameData
}

impl CharacterTableKeyFrame {
  fn into_operator_promotion_attributes(self) -> OperatorPromotionAttributes {
    OperatorPromotionAttributes {
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

  fn into_operator_trust_attributes(self) -> OperatorTrustAttributes {
    OperatorTrustAttributes {
      max_hp: self.data.max_hp,
      atk: self.data.atk,
      def: self.data.def
    }
  }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct CharacterTableKeyFrameData {
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
enum CharacterTablePosition {
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
struct CharacterTableSkill {
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
    let skill_table_entry = skill_table.get(&id)?;
    let (name, activation, recovery) = skill_table_entry.name_activation_recovery()?;
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

#[derive(Debug, Clone, Deserialize)]
struct CharacterTableSkillMastery {
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
struct CharacterTableTalent {
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
struct CharacterTableTalentCandidate {
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
struct CharacterTableTalentBlackboard {
  key: String,
  value: f32
}

impl CharacterTableTalentBlackboard {
  fn convert(blackboard: Vec<Self>) -> HashMap<String, f32> {
    recollect(blackboard, |item| (item.key, item.value))
  }
}

#[derive(Debug, Clone, Deserialize)]
struct CharacterTablePotentialRank {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
enum CharacterTableProfession {
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
  fn into_profession(self) -> Option<Profession> {
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
enum CharacterTableSubProfession {
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
  fn into_sub_profession(self) -> Option<SubProfession> {
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
