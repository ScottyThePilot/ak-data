use chrono::{DateTime, Utc};

use crate::format::*;
use crate::game_data::{HeadhuntingBanner, HeadhuntingBannerType};

use std::collections::HashMap;

impl DataFile for GachaTable {
  const LOCATION: &'static str = "excel/gacha_table.json";
  const IDENTIFIER: &'static str = "gacha_table";
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GachaTable {
  #[serde(rename = "gachaTags")]
  recruit_tags: Vec<GachaTableRecruitTag>,
  #[serde(rename = "gachaPoolClient")]
  gacha_table_client: Vec<GachaTableGachaPool>
}

impl GachaTable {
  pub(super) fn into_gacha(self) -> Gacha {
    let recruitment_tags = recollect(self.recruit_tags, GachaTableRecruitTag::into_entry);
    let headhunting_banners = recollect(self.gacha_table_client, |gacha_pool| {
      let headhunting_banner = gacha_pool.into_headhunting_banner();
      (headhunting_banner.id.clone(), headhunting_banner)
    });

    Gacha { recruitment_tags, headhunting_banners }
  }
}

#[derive(Debug, Clone, Deserialize)]
struct GachaTableRecruitTag {
  #[serde(rename = "tagId")] id: u32,
  #[serde(rename = "tagName")] name: String
}

impl GachaTableRecruitTag {
  fn into_entry(self) -> (String, u32) {
    (self.name, self.id)
  }
}

#[derive(Debug, Clone, Deserialize)]
struct GachaTableGachaPool {
  #[serde(rename = "gachaPoolId")]
  gacha_pool_id: String,
  #[serde(rename = "gachaIndex")]
  gacha_index: u32,
  #[serde(rename = "openTime")]
  #[serde(with = "chrono::serde::ts_seconds")]
  open_time: DateTime<Utc>,
  #[serde(rename = "endTime")]
  #[serde(with = "chrono::serde::ts_seconds")]
  end_time: DateTime<Utc>,
  #[serde(rename = "gachaPoolName")]
  gacha_pool_name: String,
  #[serde(rename = "gachaPoolSummary")]
  gacha_pool_summary: String,
  #[serde(rename = "LMTGSID")]
  data_contract_item_id: Option<String>,
  #[serde(rename = "gachaRuleType")]
  gacha_rule_type: GachaTableGachaRuleType
}

impl GachaTableGachaPool {
  fn into_headhunting_banner(self) -> HeadhuntingBanner {
    HeadhuntingBanner {
      id: self.gacha_pool_id,
      name: self.gacha_pool_name,
      summary: self.gacha_pool_summary,
      index: self.gacha_index,
      open_time: self.open_time,
      close_time: self.end_time,
      item_id: self.data_contract_item_id,
      banner_type: self.gacha_rule_type.into_headhunting_banner_type()
    }
  }
}

#[repr(u8)]
#[derive(Debug, Clone, Deserialize)]
enum GachaTableGachaRuleType {
  #[serde(rename = "NORMAL")]
  Normal,
  #[serde(rename = "LIMITED")]
  Limited,
  #[serde(rename = "LINKAGE")]
  Linkage,
  #[serde(rename = "ATTAIN")]
  Attain
}

impl GachaTableGachaRuleType {
  fn into_headhunting_banner_type(self) -> HeadhuntingBannerType {
    match self {
      GachaTableGachaRuleType::Normal => HeadhuntingBannerType::Normal,
      GachaTableGachaRuleType::Limited => HeadhuntingBannerType::Limited,
      GachaTableGachaRuleType::Linkage => HeadhuntingBannerType::Special,
      GachaTableGachaRuleType::Attain => HeadhuntingBannerType::Special
    }
  }
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct Gacha {
  pub(super) recruitment_tags: HashMap<String, u32>,
  pub(super) headhunting_banners: HashMap<String, HeadhuntingBanner>
}
