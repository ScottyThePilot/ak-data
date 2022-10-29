use uord::UOrd;

use crate::format::DataFile;

use std::collections::HashMap;

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
  pub(super) fn into_alters(self) -> Vec<UOrd<String>> {
    self.sp_char_groups.into_values()
      .filter_map(|value| <[String; 2]>::try_from(value).ok())
      .map(|[a, b]| UOrd::new(a, b))
      .collect()
  }
}
