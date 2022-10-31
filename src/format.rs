mod activity_table;
mod building_data;
mod character_meta_table;
mod character_table;
mod equip_table;
mod gacha_table;
mod handbook_info_table;
mod item_table;
mod range_table;
mod skill_table;

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::{Regex, Captures};
use serde::de::{Deserialize, DeserializeOwned, Deserializer};

use self::activity_table::ActivityTable;
use self::building_data::BuildingData;
use self::character_meta_table::CharacterMetaTable;
use self::character_table::CharacterTable;
use self::equip_table::EquipTable;
use self::gacha_table::GachaTable;
use self::handbook_info_table::HandbookInfoTable;
use self::item_table::ItemTable;
use self::range_table::RangeTable;
use self::skill_table::SkillTable;
use crate::game_data::{GameData, Promotion, PromotionAndLevel};
use crate::options::Options;

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;



type DataFilesTuple = (
  ActivityTable,
  BuildingData,
  CharacterMetaTable,
  CharacterTable,
  EquipTable,
  GachaTable,
  HandbookInfoTable,
  ItemTable,
  RangeTable,
  SkillTable
);

pub(crate) struct DataFiles {
  activity_table: ActivityTable,
  building_data: BuildingData,
  character_meta_table: CharacterMetaTable,
  character_table: CharacterTable,
  equip_table: EquipTable,
  gacha_table: GachaTable,
  handbook_info_table: HandbookInfoTable,
  item_table: ItemTable,
  range_table: RangeTable,
  skill_table: SkillTable
}

impl DataFiles {
  pub(crate) async fn from_local(gamedata_dir: &Path) -> Result<Self, crate::Error> {
    tokio::try_join!(
      crate::options::get_data_file_local::<ActivityTable>(gamedata_dir),
      crate::options::get_data_file_local::<BuildingData>(gamedata_dir),
      crate::options::get_data_file_local::<CharacterMetaTable>(gamedata_dir),
      crate::options::get_data_file_local::<CharacterTable>(gamedata_dir),
      crate::options::get_data_file_local::<EquipTable>(gamedata_dir),
      crate::options::get_data_file_local::<GachaTable>(gamedata_dir),
      crate::options::get_data_file_local::<HandbookInfoTable>(gamedata_dir),
      crate::options::get_data_file_local::<ItemTable>(gamedata_dir),
      crate::options::get_data_file_local::<RangeTable>(gamedata_dir),
      crate::options::get_data_file_local::<SkillTable>(gamedata_dir)
    ).map(Self::from)
  }

  pub(crate) async fn from_remote(options: &Options) -> Result<Self, crate::Error> {
    tokio::try_join!(
      crate::options::get_data_file_remote::<ActivityTable>(options),
      crate::options::get_data_file_remote::<BuildingData>(options),
      crate::options::get_data_file_remote::<CharacterMetaTable>(options),
      crate::options::get_data_file_remote::<CharacterTable>(options),
      crate::options::get_data_file_remote::<EquipTable>(options),
      crate::options::get_data_file_remote::<GachaTable>(options),
      crate::options::get_data_file_remote::<HandbookInfoTable>(options),
      crate::options::get_data_file_remote::<ItemTable>(options),
      crate::options::get_data_file_remote::<RangeTable>(options),
      crate::options::get_data_file_remote::<SkillTable>(options)
    ).map(Self::from)
  }

  pub(crate) fn into_game_data(mut self, last_updated: Option<DateTime<Utc>>) -> GameData {
    let alters = self.character_meta_table.into_alters();
    let operators = recollect_filter(self.character_table, |(id, character)| {
      Some((id.clone(), {
        character.into_operator(
          id.clone(),
          &self.building_data,
          &self.skill_table,
          &mut self.handbook_info_table,
          &mut self.equip_table
        )?
      }))
    });

    let items = self.item_table.into_items();
    let buildings = self.building_data.into_buildings();
    let ranges = recollect_map(self.range_table, |entry| entry.into_attack_range());
    let (recruitment_tags, mut headhunting_banners) = self.gacha_table.into_tags_and_banners();
    let mut events = self.activity_table.into_events();
    headhunting_banners.sort_unstable_by_key(|banner| banner.open_time);
    events.sort_unstable_by_key(|event| event.open_time);

    GameData {
      last_updated,
      alters,
      operators,
      items,
      buildings,
      ranges,
      recruitment_tags,
      headhunting_banners,
      events
    }
  }
}

impl From<DataFilesTuple> for DataFiles {
  fn from((at, bd, cmt, ct, et, gt, hbit, it, rt, st): DataFilesTuple) -> Self {
    DataFiles {
      activity_table: at,
      building_data: bd,
      character_meta_table: cmt,
      character_table: ct,
      equip_table: et,
      gacha_table: gt,
      handbook_info_table: hbit,
      item_table: it,
      range_table: rt,
      skill_table: st
    }
  }
}

pub(crate) trait DataFile: DeserializeOwned {
  const LOCATION: &'static str;
  const IDENTIFIER: &'static str;
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
struct ItemCost {
  #[serde(rename = "id")]
  item_id: String,
  count: u32
}

impl ItemCost {
  fn convert(item_cost: Vec<Self>) -> HashMap<String, u32> {
    recollect(item_cost, |item| (item.item_id, item.count))
  }
}

#[derive(Debug, Clone, Deserialize)]
struct CharCondition {
  phase: u32,
  level: u32
}

impl CharCondition {
  fn into_promotion_and_level(self) -> PromotionAndLevel {
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

static RX_TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[@$\w.]+>|</>").unwrap());
static RX_TEMPLATE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{[\w:.%\-@\[\]]+\}").unwrap());

fn strip_tags(text: &str) -> Cow<str> {
  RX_TAG.replace_all(text, "")
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
      key.to_uppercase()
    }
  });

  text.into_owned()
}

fn strip_formatting_markers(string: &str) -> (String, bool, FormattingSuffix) {
  let (negative, string) = match string.strip_prefix('-') {
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
    if string.ends_with('0') { string.pop(); };
    if string.ends_with('0') { string.pop(); };
    if string.ends_with('.') { string.pop(); };
    string
  }

  let value = if negative { -value } else { value };
  match suffix {
    FormattingSuffix::DecimalPercent => r(format!("{:.2}%", value * 100.0)),
    FormattingSuffix::IntegerPercent => format!("{:.0}%", value * 100.0),
    FormattingSuffix::Decimal => r(format!("{value:.2}")),
    FormattingSuffix::Integer => format!("{value:.0}"),
    FormattingSuffix::None => format!("{value}")
  }
}

#[derive(Debug, Clone, Copy)]
enum FormattingSuffix {
  DecimalPercent, // :0.0%
  IntegerPercent, // :0%
  Decimal, // :0.0
  Integer, // :0
  None
}

fn recollect<T, U, I, C, F>(i: I, f: F) -> C
where I: IntoIterator<Item = T>, C: FromIterator<U>, F: FnMut(T) -> U {
  i.into_iter().map(f).collect()
}

fn recollect_map<K, V, W, I, C, F>(i: I, mut f: F) -> C
where I: IntoIterator<Item = (K, V)>, C: FromIterator<(K, W)>, F: FnMut(V) -> W {
  i.into_iter().map(move |(k, v)| (k, f(v))).collect()
}

fn recollect_maybe<T, U, I, C, F>(i: I, f: F) -> Option<C>
where I: IntoIterator<Item = T>, C: FromIterator<U>, F: FnMut(T) -> Option<U> {
  recollect(i, f)
}

fn recollect_filter<T, U, I, C, F>(i: I, f: F) -> C
where I: IntoIterator<Item = T>, C: FromIterator<U>, F: FnMut(T) -> Option<U> {
  i.into_iter().filter_map(f).collect()
}
