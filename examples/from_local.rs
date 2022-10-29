use ak_data::GameData;

use std::path::PathBuf;

#[tokio::main]
async fn main() {
  let path = std::env::args_os().nth(1).map(PathBuf::from);
  let path = path.unwrap_or_else(|| PathBuf::from("../en_US/gamedata"));

  // Creates a `GameData` instance from local files
  let game_data = GameData::from_local(path)
    .await.expect("failed to get game data");
  let kroos_food_item = game_data.find_item("Vegetable Radish Tin").unwrap();
  let kroos_food_item_description = kroos_food_item.description.as_deref().unwrap();
  println!("{kroos_food_item_description}");
}
