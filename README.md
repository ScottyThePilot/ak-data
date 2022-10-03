# Arknights-Data

Rust library for parsing datamined game files from Arknights and exposing them as easy to understand Rust structures.

Currently this crate accomplishes the following things:
- Parsing operator info from `excel/character_table.json`.
  - Parsing alternate operators from `excel/char_meta_table.json`.
  - Parsing operator file records from `excel/handbook_info_table.json`.
  - Parsing operator skill info from `excel/skill_table.json`.
  - Parsing operator base skill info from `excel/building_data.json`.
- Parsing building info from `excel/building_data.json`.
- Parsing the item list from `excel/item_table.json`.

Since unobtainable characters, static map objects and 'drone' characters are included
in `excel/character_table.json`, this library filters them out for simplicity.

If you are not using an authorized application to perform the remote requests, you may run into 403 Forbidden errors
due to GitHub ratelimiting you. You can instead use `GameData::from_local` to parse local game files.

## Examples

```rust
// Creates a `GameData` instance from a GitHub repo
// (by default Kengxxiao/ArknightsGameData) and specifies the JP region
let options = Options::default().region(Region::JaJP);
let game_data = GameData::from_remote(&options).await.unwrap();
// Loops through all operators, prints their name and ID if they are from the nation "laterano"
for (id, operator) in game_data.operators.iter() {
  if let Some("laterano") = operator.nation_id.as_deref() {
    println!("{} ({id})", operator.name);
  };
};
```

```rust
// Creates a `GameData` instance from local files
let game_data = GameData::from_local("../en_US/gamedata").await.unwrap();
let kroos_food_item = game_data.find_item("Vegetable Radish Tin").unwrap();
let kroos_food_item_description = kroos_food_item.description.as_deref().unwrap();
println!("{kroos_food_item_description}");
```
