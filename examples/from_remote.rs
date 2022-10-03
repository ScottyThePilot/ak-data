use ak_data::{GameData, Options};

#[tokio::main]
async fn main() {
  // Creates a `GameData` instance from a GitHub repo
  // (by default Kengxxiao/ArknightsGameData with the en_US region)
  let options = Options::default();
  let game_data = GameData::from_remote(&options)
    .await.expect("failed to get game data");
  // Loops through all operators, prints their name and ID if they are from the nation "laterano"
  for (id, operator) in game_data.operators.iter() {
    if let Some("laterano") = operator.nation_id.as_deref() {
      println!("{} ({id})", operator.name);
    };
  };

  // Write the game data to a text file
  tokio::fs::write("game_data.txt", format!("{game_data:#?}"))
    .await.expect("failed to save game data");
}
