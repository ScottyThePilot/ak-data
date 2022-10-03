use ak_data::GameData;

#[tokio::main]
async fn main() {
  // Creates a `GameData` instance from local files
  let game_data = GameData::from_local("../en_US/gamedata")
    .await.expect("failed to get game data");
  let kroos_food_item = game_data.find_item("Vegetable Radish Tin").unwrap();
  let kroos_food_item_description = kroos_food_item.description.as_deref().unwrap();
  println!("{kroos_food_item_description}");
}
