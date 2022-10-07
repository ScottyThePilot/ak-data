#![cfg(test)]

use ak_data::game_data::{PromotionAndLevel, Promotion, OperatorPromotions};

#[test]
fn operator_promotion_attributes() {
  // tests that `OperatorPromotion::get_level_attributes` works properly
  // by comparing its results to numbers pulled from the game
  // uses sample files produced by ak-auto

  macro_rules! sample {
    ($file:expr) => (serde_json::from_slice::<OperatorPromotions>(include_bytes!($file)).unwrap());
  }

  fn test_sample(operator_promotions: OperatorPromotions, data: &[(Promotion, u32, u32, u32, u32)]) {
    for &(promotion, level, max_hp, atk, def) in data {
      let operator_promotion = operator_promotions.get(promotion).unwrap();
      let attributes = operator_promotion.get_level_attributes(level);
      assert_eq!(attributes.max_hp, max_hp);
      assert_eq!(attributes.atk, atk);
      assert_eq!(attributes.def, def);
    };
  }

  test_sample(sample!("samples/durin_promotions.json"), &[
    (Promotion::None, 1, 571, 238, 36),
    (Promotion::None, 15, 755, 287, 49),
    (Promotion::None, 30, 952, 340, 62)
  ]);

  test_sample(sample!("samples/melantha_promotions.json"), &[
    (Promotion::None, 1, 1395, 396, 83),
    (Promotion::None, 15, 1610, 463, 96),
    (Promotion::None, 25, 1763, 511, 105),
    (Promotion::None, 40, 1993, 583, 119),
    (Promotion::Elite1, 1, 1993, 583, 119),
    (Promotion::Elite1, 15, 2188, 623, 128),
    (Promotion::Elite1, 35, 2466, 681, 142),
    (Promotion::Elite1, 55, 2745, 738, 155)
  ]);

  test_sample(sample!("samples/frostleaf_promotions.json"), &[
    (Promotion::None, 1, 949, 272, 154),
    (Promotion::None, 20, 1125, 327, 179),
    (Promotion::None, 45, 1356, 400, 211),
    (Promotion::Elite1, 1, 1356, 400, 211),
    (Promotion::Elite1, 20, 1494, 443, 229),
    (Promotion::Elite1, 40, 1640, 489, 249),
    (Promotion::Elite1, 60, 1785, 534, 268),
    (Promotion::Elite2, 1, 1785, 534, 268),
    (Promotion::Elite2, 20, 1916, 569, 283),
    (Promotion::Elite2, 50, 2122, 623, 307),
    (Promotion::Elite2, 70, 2260, 660, 323)
  ]);
}



#[test]
fn promotion_and_level_ordering() {
  let sample = [
    PromotionAndLevel { promotion: Promotion::None, level: 1 },
    PromotionAndLevel { promotion: Promotion::None, level: 30 },
    PromotionAndLevel { promotion: Promotion::Elite1, level: 1 },
    PromotionAndLevel { promotion: Promotion::Elite1, level: 45 },
    PromotionAndLevel { promotion: Promotion::Elite1, level: 60 },
    PromotionAndLevel { promotion: Promotion::Elite2, level: 1 },
    PromotionAndLevel { promotion: Promotion::Elite2, level: 75 }
  ];

  for slice in sample.windows(2) {
    if let [a, b] = slice {
      assert!(a < b);
    };
  };
}
