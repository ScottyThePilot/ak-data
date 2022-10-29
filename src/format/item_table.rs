use crate::format::*;
use crate::game_data::*;

use std::collections::HashMap;

impl DataFile for ItemTable {
  const LOCATION: &'static str = "excel/item_table.json";
  const IDENTIFIER: &'static str = "item_table";
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ItemTable {
  items: HashMap<String, ItemTableItem>
}

impl ItemTable {
  pub(super) fn into_items(self) -> HashMap<String, Item> {
    recollect(self.items, |(id, item)| (id, item.into_item()))
  }
}

#[derive(Debug, Clone, Deserialize)]
struct ItemTableItem {
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
  fn into_item(self) -> Item {
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
enum ItemTableItemClassify {
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
  fn into_item_class(self) -> ItemClass {
    match self {
      ItemTableItemClassify::Consume => ItemClass::Consumable,
      ItemTableItemClassify::Normal => ItemClass::BasicItem,
      ItemTableItemClassify::Material => ItemClass::Material,
      ItemTableItemClassify::None => ItemClass::Other
    }
  }
}
