macro_rules! impl_deserialize_uint_enum {
  ($Type:ident, $TypeVisitor:ident, $expecting:literal, match {
    $($Pat:pat => $value:expr),* $(,)?
  }) => {
    impl<'de> serde::de::Deserialize<'de> for $Type {
      fn deserialize<D: serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct $TypeVisitor;

        impl<'de> serde::de::Visitor<'de> for $TypeVisitor {
          type Value = $Type;

          #[inline]
          fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str($expecting)
          }

          fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
          where E: serde::de::Error {
            match v {
              $($Pat => Ok($value),)+
              _ => Err(E::invalid_value(serde::de::Unexpected::Unsigned(v), &Self))
            }
          }
        }

        deserializer.deserialize_u64($TypeVisitor)
      }
    }
  };
}
