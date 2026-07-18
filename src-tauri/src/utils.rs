pub fn plist_to_json(value: &plist::Value) -> serde_json::Value {
    match value {
        plist::Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(plist_to_json).collect())
        }
        plist::Value::Dictionary(values) => serde_json::Value::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), plist_to_json(value)))
                .collect(),
        ),
        plist::Value::Boolean(value) => (*value).into(),
        plist::Value::Data(value) => format!("<{} bytes>", value.len()).into(),
        plist::Value::Date(value) => value.to_xml_format().into(),
        plist::Value::Real(value) => serde_json::Number::from_f64(*value)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        plist::Value::Integer(value) => {
            if let Some(value) = value.as_signed() {
                value.into()
            } else if let Some(value) = value.as_unsigned() {
                value.into()
            } else {
                serde_json::Value::Null
            }
        }
        plist::Value::String(value) => value.clone().into(),
        plist::Value::Uid(value) => value.get().into(),
        _ => serde_json::Value::Null,
    }
}

pub fn dict_string(dict: &plist::Dictionary, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| dict.get(key).and_then(plist::Value::as_string))
        .map(ToOwned::to_owned)
}

pub fn dict_u64(dict: &plist::Dictionary, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| {
        let value = dict.get(key)?;
        value
            .as_unsigned_integer()
            .or_else(|| {
                value
                    .as_signed_integer()
                    .and_then(|value| u64::try_from(value).ok())
            })
            .or_else(|| {
                value
                    .as_real()
                    .and_then(|value| u64::try_from(value as i64).ok())
            })
            .or_else(|| value.as_string().and_then(|value| value.parse().ok()))
    })
}

pub fn dict_f64(dict: &plist::Dictionary, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| {
        let value = dict.get(key)?;
        value
            .as_real()
            .or_else(|| value.as_signed_integer().map(|value| value as f64))
            .or_else(|| value.as_unsigned_integer().map(|value| value as f64))
            .or_else(|| value.as_string().and_then(|value| value.parse().ok()))
    })
}
