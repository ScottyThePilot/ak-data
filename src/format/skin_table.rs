use crate::format::*;
use crate::game_data::OperatorSkin;

use std::collections::HashMap;
use std::collections::hash_map::Entry;

impl DataFile for SkinTable {
  const LOCATION: &'static str = "excel/skin_table.json";
  const IDENTIFIER: &'static str = "skin_table";
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct SkinTable {
  #[serde(rename = "charSkins")]
  character_skins: HashMap<String, SkinTableCharacterSkin>,
  #[serde(rename = "buildinEvolveMap")]
  default_evolve_map: HashMap<String, SkinTableEvolutions>
}

impl SkinTable {
  pub(super) fn into_skin_table_mapped(mut self) -> SkinTableMapped {
    let mut characters = HashMap::<String, SkinTableCharacterEntry>::new();
    for (id, character_skin) in self.character_skins {
      if let Some(operator_skin) = character_skin.into_operator_skin() {
        let character_entry = match characters.entry(operator_skin.model_id.clone()) {
          Entry::Occupied(entry) => entry.into_mut(),
          Entry::Vacant(entry) => match take_default_skins(&mut self.default_evolve_map, &operator_skin.model_id) {
            Some(default_skins) => entry.insert(SkinTableCharacterEntry::new(default_skins)),
            None => continue
          }
        };

        character_entry.skins.insert(id, operator_skin);
      };
    };

    SkinTableMapped { characters }
  }
}

fn take_default_skins(default_evolve_map: &mut HashMap<String, SkinTableEvolutions>, id: &str) -> Option<[Option<String>; 3]> {
  default_evolve_map.remove(id).map(|mut default_evolutions| {
    [E0, E1, E2].map(|phase| default_evolutions.remove(&phase))
  })
}

#[derive(Debug, Clone)]
pub(super) struct SkinTableMapped {
  characters: HashMap<String, SkinTableCharacterEntry>
}

impl SkinTableMapped {
  pub(super) fn take_character_entry(&mut self, character_id: &str) -> Option<SkinTableCharacterEntry> {
    self.characters.remove(character_id)
  }
}

#[derive(Debug, Clone)]
pub(super) struct SkinTableCharacterEntry {
  pub(super) skins: crate::Map<String, OperatorSkin>,
  pub(super) default_skins: [Option<String>; 3]
}

impl SkinTableCharacterEntry {
  fn new(default_skins: [Option<String>; 3]) -> Self {
    SkinTableCharacterEntry {
      skins: crate::Map::new(),
      default_skins
    }
  }
}

#[derive(Debug, Clone, Deserialize)]
struct SkinTableCharacterSkin {
  #[serde(rename = "skinId")]
  id: String,
  #[serde(rename = "charId")]
  character_id: String,
  #[serde(rename = "illustId")]
  illustration_id: Option<String>, // Always some for valid skins
  #[serde(rename = "dynIllustId")]
  illustration_live_id: Option<String>,
  #[serde(rename = "avatarId")]
  avatar_id: Option<String>, // Always some for valid skins
  #[serde(rename = "portraitId")]
  portrait_id: Option<String>, // Always some for valid skins
  #[serde(rename = "isBuySkin")]
  is_paid_skin: bool,
  #[serde(rename = "displaySkin")]
  display_skin: SkinTableDisplaySkin
}

impl SkinTableCharacterSkin {
  fn into_operator_skin(self) -> Option<OperatorSkin> {
    let dialog = self.display_skin.dialog
      .or_else(|| self.display_skin.content)
      .map(|dialog| strip_tags(&dialog).into_owned());
    Some(OperatorSkin {
      id: self.id.clone(),
      name: self.display_skin.name,
      model_id: self.character_id,
      model_name: self.display_skin.model_name?,
      is_paid: self.is_paid_skin,
      illustration_id: self.illustration_id?,
      illustration_live_id: self.illustration_live_id,
      avatar_id: self.avatar_id?,
      portrait_id: self.portrait_id?,
      illustrator: self.display_skin.illustrator?,
      group: self.display_skin.group?,
      dialog,
      usage: self.display_skin.usage,
      description: self.display_skin.description,
      obtain: self.display_skin.obtain
    })
  }
}

#[derive(Debug, Clone, Deserialize)]
struct SkinTableDisplaySkin {
  #[serde(rename = "skinName")]
  name: Option<String>, // Will be none if default outfit
  #[serde(rename = "modelName")]
  model_name: Option<String>,
  #[serde(rename = "drawerName")]
  illustrator: Option<String>,
  #[serde(rename = "skinGroupName")]
  group: Option<String>,
  content: Option<String>,
  dialog: Option<String>,
  usage: Option<String>,
  description: Option<String>,
  #[serde(rename = "obtainApproach")]
  obtain: Option<String>
}

type SkinTableEvolutions = HashMap<SkinTableEvolvePhase, String>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
enum SkinTableEvolvePhase {
  #[serde(rename = "0")]
  Elite0,
  #[serde(rename = "1")]
  Elite1,
  #[serde(rename = "2")]
  Elite2
}

const E0: SkinTableEvolvePhase = SkinTableEvolvePhase::Elite0;
const E1: SkinTableEvolvePhase = SkinTableEvolvePhase::Elite1;
const E2: SkinTableEvolvePhase = SkinTableEvolvePhase::Elite2;
