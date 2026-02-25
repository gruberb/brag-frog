use serde::Deserialize;

/// Deserializes an HTML form field to `Option<i64>`: empty string becomes `None`.
pub fn deserialize_optional_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.is_empty() {
        Ok(None)
    } else {
        s.parse::<i64>().map(Some).map_err(serde::de::Error::custom)
    }
}

/// Deserializes an HTML form field to `Option<String>`: blank/whitespace becomes `None`.
pub fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}
