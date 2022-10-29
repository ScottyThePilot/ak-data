use mint::Point2;

use crate::format::*;
use crate::game_data::AttackRange;

use std::collections::HashMap;

impl DataFile for RangeTable {
  const LOCATION: &'static str = "excel/range_table.json";
  const IDENTIFIER: &'static str = "range_table";
}

pub(crate) type RangeTable = HashMap<String, RangeTableEntry>;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RangeTableEntry {
  // omitted `direction`, it seems to only be 1 for every entry
  grids: Vec<RangeTableGridPoint>
}

impl RangeTableEntry {
  pub(super) fn into_attack_range(self) -> AttackRange {
    AttackRange { points: recollect(self.grids, RangeTableGridPoint::into_point2) }
  }
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct RangeTableGridPoint {
  row: i32,
  col: i32
}

impl RangeTableGridPoint {
  fn into_point2(self) -> Point2<i32> {
    Point2 { x: self.col, y: self.row }
  }
}
