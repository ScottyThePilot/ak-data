//! Structs containing information parsed from Arknights' game files.
//! The main entrypoint for accessing any of these items is [`GameData`].
//!
//! See the examples for usage help.

use chrono::{DateTime, Utc};
use mint::Point2;
pub use uord::UOrd;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Iter as HashMapIter;
use std::iter::{Chain, Once};
use std::num::NonZeroU8;
use std::option::IntoIter as OptionIter;
use std::ops::Deref;
use std::path::Path;

use crate::options::Options;



/// Encapsulates game data extracted from Arknights' game files.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameData {
  /// The time this GameData was updated, if it was created from a remote source.
  pub last_updated: Option<DateTime<Utc>>,
  /// Lists all of the pairs of alternate operators that exist.
  pub alters: Vec<UOrd<String>>,
  /// A list of all obtainable operators in the game.
  pub operators: HashMap<String, Operator>,
  /// A list of all items in the game.
  pub items: HashMap<String, Item>,
  /// A list of all RIIC base buildings.
  pub buildings: HashMap<BuildingType, Building>,
  /// A list of all operator attack ranges.
  pub ranges: HashMap<String, AttackRange>,
  /// A list of all recruitment tags.
  pub recruitment_tags: HashMap<String, u32>,
  /// A list of all past, current and future banners according to the game files.
  pub headhunting_banners: Vec<HeadhuntingBanner>,
  /// A list of all past, current and future events according to the game files.
  pub events: HashMap<String, Event>
}

impl GameData {
  /// Tries constructing a [`GameData`] instance from the given path.
  /// Note that the provided path should go to the `gamedata` folder, not the root folder of the repository.
  pub async fn from_local<P: AsRef<Path>>(path: P) -> Result<Self, crate::Error> {
    let data_files = crate::format::DataFiles::from_local(path.as_ref()).await?;
    Ok(data_files.into_game_data(None))
  }

  /// Tries constructing a [`GameData`] from a remote GitHub repository.
  /// The [`Options`] instance will dictate which repository to fetch from.
  pub async fn from_remote(options: &Options) -> Result<Self, crate::Error> {
    options.request_game_data().await
  }

  /// Patches this [`GameData`] if the data it is based on is out of date.
  /// Replaces `self` and returns it if it was out of date.
  pub async fn patch_from_remote(&mut self, options: &Options) -> Result<Option<Self>, crate::Error> {
    options.patch_game_data(self).await
  }

  /// Returns true if the given date time is more recent than the update time included in this game data.
  pub fn is_outdated(&self, new_date_time: DateTime<Utc>) -> bool {
    self.last_updated.map_or(true, |last_updated| last_updated < new_date_time)
  }

  /// Takes an operator ID, returns the operator ID if an alter exists corresponding to it.
  pub fn get_alter_for(&self, operator: &str) -> Option<&str> {
    self.alters.iter()
      .find_map(|alter_group| alter_group.other(operator))
      .map(String::as_str)
  }

  /// Searches for an operator, given their in-game name.
  /// Please remember that names are region dependent!
  pub fn find_operator(&self, operator_name: impl AsRef<str>) -> Option<&Operator> {
    let operator_name = operator_name.as_ref();
    self.operators.values().find(|&operator| {
      operator.name.eq_ignore_ascii_case(operator_name)
    })
  }

  /// Searches for an item, given its in-game name.
  /// Please remember that names are region dependent!
  pub fn find_item(&self, item_name: impl AsRef<str>) -> Option<&Item> {
    let item_name = item_name.as_ref();
    self.items.values().find(|&item| {
      item.name.eq_ignore_ascii_case(item_name)
    })
  }

  /// Returns an iterator over all headhunting banners that have passed.
  #[inline]
  pub fn iter_past_banners(&self, now: DateTime<Utc>) -> impl Iterator<Item = &HeadhuntingBanner> {
    self.headhunting_banners.iter().filter(move |banner| banner.is_past(now))
  }

  /// Returns an iterator over all headhunting banners that are currently open.
  #[inline]
  pub fn iter_current_banners(&self, now: DateTime<Utc>) -> impl Iterator<Item = &HeadhuntingBanner> {
    self.headhunting_banners.iter().filter(move |banner| banner.is_current(now))
  }

  /// Returns an iterator over all headhunting banners that have yet to open, usually empty.
  #[inline]
  pub fn iter_future_banners(&self, now: DateTime<Utc>) -> impl Iterator<Item = &HeadhuntingBanner> {
    self.headhunting_banners.iter().filter(move |banner| banner.is_future(now))
  }

  /// Returns an iterator over all events that have passed.
  #[inline]
  pub fn iter_past_events(&self, now: DateTime<Utc>) -> impl Iterator<Item = &Event> {
    self.events.values().filter(move |banner| banner.is_past(now))
  }

  /// Returns an iterator over all events that are currently open.
  #[inline]
  pub fn iter_current_events(&self, now: DateTime<Utc>) -> impl Iterator<Item = &Event> {
    self.events.values().filter(move |banner| banner.is_current(now))
  }

  /// Returns an iterator over all events that have yet to open.
  #[inline]
  pub fn iter_future_events(&self, now: DateTime<Utc>) -> impl Iterator<Item = &Event> {
    self.events.values().filter(move |banner| banner.is_future(now))
  }
}

/// An operator.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operator {
  /// This operator's hidden ID.
  pub id: String,
  /// This operator's name, region dependent.
  pub name: String,
  /// The nation this operator belongs to, region independent. (Example: `"victoria"` for Bagpipe)
  pub nation_id: Option<String>,
  /// The group this operator belongs to, region independent. (Example: `"karlan"` for SilverAsh)
  pub group_id: Option<String>,
  /// The group this operator belongs to, region independent. (Example: `"reserve4"` for Adnachiel)
  pub team_id: Option<String>,
  /// A three or four letter code that is displayed in the in-game archive screen. (Example: `"LT77"` for Mostima)
  pub display_number: String,
  /// Appears to be for an 'alternate name' like the Ursus operators' cyrllic names.
  /// (On non-EN regions, the appellation will be the operator's name in latin script)
  pub appellation: Option<String>,
  /// Whether this operator is ranged or melee.
  /// (Note that operators that can do both may show as melee)
  pub position: Position,
  /// The recruitment tags for this operator, region dependent.
  pub recruitment_tags: Vec<String>,
  /// Ranges from 1 to 6, indicates the number of stars (rarity) of this operator.
  pub rarity: NonZeroU8,
  /// The operator's primary profession.
  pub profession: Profession,
  /// The operator's secondary sub-profession.
  pub sub_profession: SubProfession,
  /// A list of promotions that this operator can achieve.
  pub promotions: OperatorPromotions,
  /// The item required to upgrade this operator's potential.
  pub potential_item_id: Option<String>,
  /// Will almost always be length 5.
  /// Exceptions are Savage and any operators without potential.
  pub potential: Vec<OperatorPotential>,
  /// A list of skills and their upgrade phases that this operator can achieve.
  pub skills: Vec<OperatorSkill>,
  /// A list of talents and their unlock phases that this operator can achieve.
  pub talents: Vec<OperatorTalent>,
  /// The list of non-default modules for this operator.
  pub modules: Vec<OperatorModule>,
  pub base_skills: Vec<OperatorBaseSkill>,
  pub trust_bonus: OperatorTrustAttributes,
  pub file: OperatorFile
}

impl Operator {
  pub fn get_potential_item<'a>(&self, items: &'a HashMap<String, Item>) -> Option<&'a Item> {
    self.potential_item_id.as_deref().and_then(|item_id| items.get(item_id))
  }
}

/// Contains information about an operator's three possible promotion phases.
/// The default (none) promotion, elite level 1, and elite level 2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorPromotions {
  pub none: OperatorPromotion,
  pub elite1: Option<OperatorPromotion>,
  pub elite2: Option<OperatorPromotion>
}

impl OperatorPromotions {
  /// Obtains a reference to the respective [`OperatorPromotion`], given a [`Promotion`].
  pub fn get(&self, promotion: Promotion) -> Option<&OperatorPromotion> {
    match promotion {
      Promotion::None => Some(&self.none),
      Promotion::Elite1 => self.elite1.as_ref(),
      Promotion::Elite2 => self.elite2.as_ref()
    }
  }

  /// Returns the stats of this operator at the given promotion and level.
  /// (Does not account for stat boosts from talents.)
  pub fn get_attributes(&self, promotion_and_level: PromotionAndLevel) -> Option<OperatorPromotionAttributes> {
    let PromotionAndLevel { promotion, level } = promotion_and_level;
    self.get(promotion).map(|promotion| promotion.get_level_attributes(level))
  }

  #[inline]
  pub fn iter(&self) -> OperatorPromotionsIter<&OperatorPromotion> {
    self.into_iter()
  }
}

/// Iterates over between 1 and 3 items of type `P`.
pub type OperatorPromotionsIter<P> = Chain<Chain<Once<P>, OptionIter<P>>, OptionIter<P>>;

impl IntoIterator for OperatorPromotions {
  type Item = OperatorPromotion;
  type IntoIter = OperatorPromotionsIter<OperatorPromotion>;

  fn into_iter(self) -> Self::IntoIter {
    std::iter::once(self.none)
      .chain(self.elite1)
      .chain(self.elite2)
  }
}

impl<'a> IntoIterator for &'a OperatorPromotions {
  type Item = &'a OperatorPromotion;
  type IntoIter = OperatorPromotionsIter<&'a OperatorPromotion>;

  fn into_iter(self) -> Self::IntoIter {
    std::iter::once(&self.none)
      .chain(self.elite1.as_ref())
      .chain(self.elite2.as_ref())
  }
}

/// An unlockable promotion level for an operator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorPromotion {
  pub attack_range_id: Option<String>,
  pub min_attributes: OperatorPromotionAttributes,
  pub max_attributes: OperatorPromotionAttributes,
  pub max_level: u32,
  pub upgrade_cost: HashMap<String, u32>
}

impl OperatorPromotion {
  #[inline]
  pub fn iter_upgrade_cost<'a>(&'a self, items: &'a HashMap<String, Item>) -> ItemsIter<'a> {
    ItemsIter::new(&self.upgrade_cost, items)
  }

  pub fn get_attack_range<'a>(&self, ranges: &'a HashMap<String, AttackRange>) -> Option<&'a AttackRange> {
    self.attack_range_id.as_deref().and_then(|attack_range_id| ranges.get(attack_range_id))
  }

  pub fn get_level_attributes(&self, level: u32) -> OperatorPromotionAttributes {
    OperatorPromotionAttributes {
      level,
      max_hp: self.lerp_attribute_u32(level, |attributes| attributes.max_hp),
      atk: self.lerp_attribute_u32(level, |attributes| attributes.atk),
      def: self.lerp_attribute_u32(level, |attributes| attributes.def),
      // no other operator attributes appear to actually change with level
      ..self.min_attributes
    }
  }

  fn level_t(&self, level: u32) -> f32 {
    let min = self.min_attributes.level;
    let max = self.max_attributes.level;
    let level = level.clamp(min, max) as f32;
    let (min, max) = (min as f32, max as f32);
    (level - min) / (max - min)
  }

  fn lerp_attribute_u32<F>(&self, level: u32, f: F) -> u32
  where F: Fn(&OperatorPromotionAttributes) -> u32 {
    let min = f(&self.min_attributes);
    let max = f(&self.max_attributes);
    lerp_u32(min, max, self.level_t(level))
  }

  fn lerp_attribute_f32<F>(&self, level: u32, f: F) -> f32
  where F: Fn(&OperatorPromotionAttributes) -> f32 {
    let min = f(&self.min_attributes);
    let max = f(&self.max_attributes);
    lerp_f32(min, max, self.level_t(level))
  }
}

/// Operator attributes associated with an operator promotion.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OperatorPromotionAttributes {
  pub level: u32,
  pub max_hp: u32,
  pub atk: u32,
  pub def: u32,
  pub magic_resistance: f32,
  pub deployment_cost: u32,
  pub block_count: u8,
  pub move_speed: f32,
  pub attack_speed: f32,
  pub base_attack_time: f32,
  pub redeploy_time: u32,
  pub hp_recovery_per_sec: f32,
  pub sp_recovery_per_sec: f32,
  pub max_deploy_count: u32,
  pub max_deck_stack_count: u32,
  pub taunt_level: i8,
  pub is_stun_immune: bool,
  pub is_silence_immune: bool,
  pub is_sleep_immune: bool,
  pub is_frozen_immune: bool
}

/// Operator attributes associated with an operator's trust level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorTrustAttributes {
  pub max_hp: u32,
  pub atk: u32,
  pub def: u32
}

impl OperatorTrustAttributes {
  pub fn get_trust_level_attributes(&self, trust: u32) -> OperatorTrustAttributes {
    // trust cannot go over 200
    let t = trust.min(200) as f32 / 200.0;
    OperatorTrustAttributes {
      max_hp: lerp_u32(0, self.max_hp, t),
      atk: lerp_u32(0, self.atk, t),
      def: lerp_u32(0, self.def, t)
    }
  }
}

impl Default for OperatorTrustAttributes {
  fn default() -> Self {
    OperatorTrustAttributes { max_hp: 0, atk: 0, def: 0 }
  }
}

#[inline]
fn lerp_f32(min: f32, max: f32, t: f32) -> f32 {
  min + (max - min) * t
}

#[inline]
fn lerp_u32(min: u32, max: u32, t: f32) -> u32 {
  lerp_f32(min as f32, max as f32, t).round() as u32
}

/// A single 'potential' upgrade level for an operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorPotential {
  /// Only two values currently appear:
  /// - `0` which corresponds to stat boosts.
  /// - `1` which improves a talent.
  pub potential_type: u32,
  pub description: String
}

/// An operator's skill and all of its upgradeable levels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorSkill {
  pub id: String,
  pub name: String,
  pub prefab_key: Option<String>,
  pub condition: PromotionAndLevel,
  pub activation: SkillActivation,
  pub recovery: SkillRecovery,
  /// Upgrade levels 1-7.
  pub levels: [OperatorSkillLevel; 7],
  /// Mastery levels 1-3 (If applicable).
  pub mastery: Option<[OperatorSkillMastery; 3]>
}

impl OperatorSkill {
  /// Returns whether or not this skill has been unlocked based on the given promotion and level.
  pub fn is_unlocked(&self, promotion_and_level: PromotionAndLevel) -> bool {
    self.condition <= promotion_and_level
  }
}

/// An upgradeable level of an operator's skill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorSkillLevel {
  pub description: Option<String>,
  pub attack_range_id: Option<String>,
  pub prefab_key: Option<String>,
  pub duration: f32,
  pub max_charge_time: u32,
  pub sp_cost: u32,
  pub initial_sp: u32,
  pub increment: f32
}

impl OperatorSkillLevel {
  pub fn get_attack_range<'a>(&self, ranges: &'a HashMap<String, AttackRange>) -> Option<&'a AttackRange> {
    self.attack_range_id.as_deref().and_then(|attack_range_id| ranges.get(attack_range_id))
  }
}

/// An upgradeable mastery level of an operator's skill.
///
/// Implements `Deref<Target = OperatorSkillLevel>` so that you can access
/// the fields of [`OperatorSkillLevel`] directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorSkillMastery {
  pub condition: PromotionAndLevel,
  pub upgrade_time: u32,
  pub upgrade_cost: HashMap<String, u32>,
  pub level: OperatorSkillLevel
}

impl Deref for OperatorSkillMastery {
  type Target = OperatorSkillLevel;

  #[inline]
  fn deref(&self) -> &Self::Target {
    &self.level
  }
}

impl OperatorSkillMastery {
  /// Returns whether or not this mastery's promotion and level requirements have been met
  pub fn is_unlockable(&self, promotion_and_level: PromotionAndLevel) -> bool {
    self.condition <= promotion_and_level
  }

  #[inline]
  pub fn iter_upgrade_cost<'a>(&'a self, items: &'a HashMap<String, Item>) -> ItemsIter<'a> {
    ItemsIter::new(&self.upgrade_cost, items)
  }
}

/// The activation mode of an operator's skill.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SkillActivation {
  Passive,
  Manual,
  Auto
}

/// The recovery mode of an operator's skill.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SkillRecovery {
  Passive,
  AutoRecovery,
  OffensiveRecovery,
  DefensiveRecovery
}

/// An operator's talent and all of its unlockable phases.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorTalent {
  pub phases: Vec<OperatorTalentPhase>
}

impl OperatorTalent {
  /// Provided a promotion, level and potential level, return the respective unlocked talent phase.
  pub fn get_unlocked(&self, promotion_and_level: PromotionAndLevel, potential: u8) -> Option<&OperatorTalentPhase> {
    self.phases.iter().rev().find(|phase| phase.is_unlocked(promotion_and_level, potential))
  }
}

/// An unlockable phase of an operator's talent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorTalentPhase {
  pub name: String,
  pub description: String,
  pub condition: PromotionAndLevel,
  pub required_potential: u8,
  /// I don't know what this key does, however I can say the following things about it:
  ///
  /// It currently has only four possible values: `1`, `1+`, `2` and `#`.
  /// - When it's `1`, it's always on the first talent.
  /// - When it's `1+`, it's always on the first talent.
  /// - When it's `2`, it's always on the second talent.
  /// - `#` is currently only present on Amiya's "???" talent and on Phantom's "Phantom Mastery" talent.
  ///   There's no discernible pattern here, maybe a "special" talent marker?
  pub prefab_key: String,
  pub attack_range_id: Option<String>,
  pub effects: HashMap<String, f32>
}

impl OperatorTalentPhase {
  pub fn is_unlocked(&self, promotion_and_level: PromotionAndLevel, potential: u8) -> bool {
    self.condition <= promotion_and_level && self.required_potential <= potential
  }

  pub fn get_attack_range<'a>(&self, ranges: &'a HashMap<String, AttackRange>) -> Option<&'a AttackRange> {
    self.attack_range_id.as_deref().and_then(|attack_range_id| ranges.get(attack_range_id))
  }
}

/// An unlockable module for an operator. Currently, no operators have more than one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorModule {
  pub id: String,
  pub name: String,
  pub description: String,
  pub condition: PromotionAndLevel,
  pub required_trust: u32,
  pub upgrade_cost: HashMap<String, u32>,
  pub missions: HashMap<String, OperatorModuleMission>
}

impl OperatorModule {
  /// Returns whether or not this module's promotion and level requirements have been met
  pub fn is_unlockable(&self, promotion_and_level: PromotionAndLevel, trust: u32) -> bool {
    self.condition <= promotion_and_level && self.required_trust <= trust
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorModuleMission {
  pub description: String,
  pub sort: u32
}

/// An operator's base skill and all of its unlockable phases.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorBaseSkill {
  pub phases: Vec<OperatorBaseSkillPhase>
}

impl OperatorBaseSkill {
  pub fn get_unlocked(&self, promotion_and_level: PromotionAndLevel) -> Option<&OperatorBaseSkillPhase> {
    self.phases.iter().rev().find(|phase| phase.is_unlocked(promotion_and_level))
  }
}

/// An unlockable phase of an operator's base skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorBaseSkillPhase {
  pub name: String,
  pub condition: PromotionAndLevel,
  pub sort: u32,
  pub category: OperatorBaseSkillCategory,
  pub building_type: BuildingType
}

impl OperatorBaseSkillPhase {
  pub fn is_unlocked(&self, promotion_and_level: PromotionAndLevel) -> bool {
    self.condition <= promotion_and_level
  }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum OperatorBaseSkillCategory {
  Function,
  Recovery,
  Output
}

/// Indicates whether an operator is primarily melee or primarily ranged.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Position {
  Melee,
  Ranged
}

/// Represents the promotion level and numeric level of an operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PromotionAndLevel {
  pub promotion: Promotion,
  pub level: u32
}

impl PartialOrd for PromotionAndLevel {
  #[inline]
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(Self::cmp(self, other))
  }
}

impl Ord for PromotionAndLevel {
  #[inline]
  fn cmp(&self, other: &Self) -> Ordering {
    Promotion::cmp(&self.promotion, &other.promotion)
      .then(u32::cmp(&self.level, &other.level))
  }
}

/// The promotion level of an operator.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Promotion {
  None = 0,
  Elite1 = 1,
  Elite2 = 2
}

impl Promotion {
  /// Add a level to this [`Promotion`] to make it a [`PromotionAndLevel`].
  pub fn with_level(self, level: u32) -> PromotionAndLevel {
    PromotionAndLevel { promotion: self, level }
  }
}

/// An operator's primary profession.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Profession {
  Caster,
  Medic,
  Vanguard,
  Sniper,
  Specialist,
  Support,
  Tank,
  Guard
}

/// An operator's secondary sub-profession.
///
/// This enum is marked as non-exhaustive because Hypergryph may add new sub-professions in the future.
#[repr(u8)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SubProfession {
  // Casters
  BlastCaster,
  ChainCaster,
  CoreCaster,
  MechAccordCaster,
  MysticCaster,
  PhalanxCaster,
  SplashCaster,
  // Medics
  Therapist,
  Medic,
  MultiTargetMedic,
  WanderingMedic,
  // Vanguards
  StandardBearer,
  Charger,
  Pioneer,
  Tactician,
  // Snipers
  Artilleryman,
  Flinger,
  Heavyshooter,
  Marksman,
  Deadeye,
  Spreadshooter,
  Besieger,
  // Specialists
  Dollkeeper,
  Executor,
  Geek,
  Hookmaster,
  Merchant,
  PushStroker,
  Ambusher,
  Trapmaster,
  // Supports
  Bard,
  Abjurer,
  Artificer,
  DecelBinder,
  Summoner,
  Hexer,
  // Tanks
  ArtsProtector,
  Duelist,
  Fortress,
  Guardian,
  Protector,
  Juggernaut,
  // Guards
  ArtsFighter,
  Centurion,
  Dreadnought,
  Fighter,
  Instructor,
  Liberator,
  Lord,
  Musha,
  Reaper,
  Swordmaster
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
  pub id: String,
  pub name: String,
  pub event_type: EventType,
  pub open_time: DateTime<Utc>,
  pub close_time: DateTime<Utc>,
  pub close_time_rewards: DateTime<Utc>,
  pub is_rerun: bool
}

impl Event {
  /// Whether this event has already opened and closed.
  pub fn is_past(&self, now: DateTime<Utc>) -> bool {
    now >= self.close_time_rewards
  }

  /// Whether this event is currently open and still has playable levels.
  pub fn is_current_playable(&self, now: DateTime<Utc>) -> bool {
    self.open_time <= now && now < self.close_time
  }

  /// Whether this event is currently open.
  /// This includes the extra phase after an event ends when the shop is still accessable.
  pub fn is_current(&self, now: DateTime<Utc>) -> bool {
    self.open_time <= now && now < self.close_time_rewards
  }

  /// Whether this event has yet to open.
  pub fn is_future(&self, now: DateTime<Utc>) -> bool {
    self.open_time > now
  }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EventType {
  /// For example: A Walk in the Dust, Darknights Memoir.
  Intermezzi,
  /// For example: Maria Nearl, Guide Ahead.
  SideStory,
  /// For example: Children of Ursus, Vigilo.
  Vignette
}

/// A headhunting banner that existed or will exist at some point according to the game data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadhuntingBanner {
  pub id: String,
  pub name: String,
  /// A string describing the time that this banner closes.
  pub summary: String,
  pub index: u32,
  pub open_time: DateTime<Utc>,
  pub close_time: DateTime<Utc>,
  /// The ID of the 'Headhunting Data Contract' item (free 10-pull item)
  /// associated with this banner, if it has one.
  pub item_id: Option<String>,
  pub banner_type: HeadhuntingBannerType
}

impl HeadhuntingBanner {
  /// Whether this banner has already opened and closed.
  pub fn is_past(&self, now: DateTime<Utc>) -> bool {
    now >= self.close_time
  }

  /// Whether this banner is currently open and has yet to close.
  pub fn is_current(&self, now: DateTime<Utc>) -> bool {
    self.open_time <= now && now < self.close_time
  }

  /// Whether this banner has yet to open.
  pub fn is_future(&self, now: DateTime<Utc>) -> bool {
    self.open_time > now
  }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HeadhuntingBannerType {
  Normal,
  /// Limited event banners.
  Limited,
  /// This corresponds with the `ATTAIN` and `LINKAGE` rules types defined in `gacha_table.json`,
  /// which so far have only appeared on the "Celebrate & Recollect" (`ATTAIN`) and the
  /// "Attack - Defence - Tactical Collide" R6S crossover banner (`LINKAGE`).
  Special
}

/// Represents an RIIC base room that can exist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Building {
  pub building_type: BuildingType,
  pub name: String,
  pub description: Option<String>,
  pub max_count: Option<u32>,
  pub category: String,
  /// Size of this room in (width, height)
  pub size: (u32, u32),
  pub upgrades: Vec<BuildingUpgrade>
}

/// Represents a potential upgrade that can be applied to an RIIC base room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingUpgrade {
  pub unlock_condition: String,
  /// Materials required to construct/upgrade this building.
  pub construction_cost: HashMap<String, u32>,
  /// Drones required to construct/upgrade this building.
  pub construction_drones: u32,
  /// The amount of power that this building consumes/produces.
  /// Will be positive for power plants, negative for other buildings.
  pub power: i32,
  pub operator_capacity: u32,
  pub manpower_cost: u32
}

impl BuildingUpgrade {
  #[inline]
  pub fn iter_construction_cost<'a>(&'a self, items: &'a HashMap<String, Item>) -> ItemsIter<'a> {
    ItemsIter::new(&self.construction_cost, items)
  }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BuildingType {
  ControlCenter,
  PowerPlant,
  Factory,
  TradingPost,
  Dormitory,
  Workshop,
  Office,
  TrainingRoom,
  ReceptionRoom,
  Elevator,
  Corridor
}

/// An item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
  pub id: String,
  pub name: String,
  pub description: Option<String>,
  pub rarity: u32,
  pub usage: Option<String>,
  pub obtain: Option<String>,
  pub item_class: ItemClass,
  pub item_type: String
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ItemClass {
  Consumable,
  BasicItem,
  Material,
  Other
}

/// Contains operator file entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorFile {
  pub id: String,
  pub illustrator_name: String,
  pub entries: Vec<OperatorFileEntry>
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorFileEntry {
  pub title: String,
  pub text: String,
  pub unlock_condition: OperatorFileUnlock
}

impl OperatorFileEntry {
  #[inline]
  fn iter_text_lines(&self) -> impl Iterator<Item = &str> + DoubleEndedIterator {
    self.text.lines().map(str::trim).filter(|line| !line.is_empty())
  }

  /// Searches for an entry line based on a bracketed header.
  ///
  /// # Example
  /// ```no_run
  /// # use ak_data::game_data::GameData;
  /// # #[tokio::main]
  /// # async fn main() {
  /// #   let game_data = GameData::from_local("gamedata").await.expect("failed to get game data");
  /// let fiammeta = game_data.find_operator("Fiammeta").expect("no fiammeta :(");
  /// assert_eq!(fiammeta.file.entries[0].find_line("Gender"), Some("Female"));
  /// # }
  /// ```
  pub fn find_line(&self, name: &str) -> Option<&str> {
    self.iter_text_lines().find_map(|line| {
      let (line_name, line_text) = split_text_line(line)?;
      if name == line_name { Some(line_text) } else { None }
    })
  }

  pub fn is_unlocked(&self, promotion_and_level: PromotionAndLevel, trust: u32) -> bool {
    self.unlock_condition.test(promotion_and_level, trust)
  }
}

fn split_text_line(line: &str) -> Option<(&str, &str)> {
  let line = line.trim();
  line.strip_prefix("[")?.split_once("] ")
}

/// The unlock condition associated with an operator file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatorFileUnlock {
  AlwaysUnlocked,
  Trust(u32),
  PromotionLevel(PromotionAndLevel),
  OperatorUnlocked(String)
}

impl OperatorFileUnlock {
  /// Will always return `false` for `OperatorUnlocked`, please handle that case manually.
  /// Currently, the only entry that uses `OperatorUnlocked` is Amiya's second-last operator file entry
  /// which only unlocks when the player owns Guard Amiya.
  pub fn test(&self, promotion_and_level: PromotionAndLevel, trust: u32) -> bool {
    match self {
      OperatorFileUnlock::AlwaysUnlocked => true,
      OperatorFileUnlock::Trust(condition) => *condition <= trust,
      OperatorFileUnlock::PromotionLevel(condition) => *condition <= promotion_and_level,
      OperatorFileUnlock::OperatorUnlocked(..) => false
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttackRange {
  pub points: HashSet<Point2<i32>>
}

impl AttackRange {
  pub fn contains(&self, point: impl Into<Point2<i32>>) -> bool {
    self.points.contains(&point.into())
  }
}

/// Iterates over [`Item`]s given a list of item IDs.
#[derive(Debug, Clone)]
pub struct ItemsIter<'a> {
  iter: HashMapIter<'a, String, u32>,
  items: &'a HashMap<String, Item>
}

impl<'a> ItemsIter<'a> {
  #[inline]
  pub fn new(list: &'a HashMap<String, u32>, items: &'a HashMap<String, Item>) -> Self {
    ItemsIter { iter: list.iter(), items }
  }

  #[inline]
  fn get(
    items: &'a HashMap<String, Item>,
    (id, &count): (&'a String, &'a u32)
  ) -> Option<(&'a Item, u32)> {
    items.get(id).map(|item| (item, count))
  }
}

impl<'a> Iterator for ItemsIter<'a> {
  type Item = (&'a Item, u32);

  #[inline]
  fn next(&mut self) -> Option<Self::Item> {
    self.iter.find_map(|value| {
      Self::get(self.items, value)
    })
  }

  #[inline]
  fn size_hint(&self) -> (usize, Option<usize>) {
    (0, self.iter.size_hint().1)
  }

  #[inline]
  fn fold<Acc, Fold>(self, init: Acc, mut fold: Fold) -> Acc
  where Fold: FnMut(Acc, Self::Item) -> Acc {
    self.iter.fold(init, |acc, value| {
      match Self::get(self.items, value) {
        Some(x) => fold(acc, x),
        None => acc
      }
    })
  }
}
