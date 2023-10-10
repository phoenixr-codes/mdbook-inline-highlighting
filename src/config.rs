use toml::Value;

#[derive(Default)]
pub(crate) struct Configuration {
    pub default_language: Option<String>,
}

impl TryFrom<&toml::map::Map<String, toml::Value>> for Configuration {
    type Error = &'static str;

    fn try_from(value: &toml::map::Map<String, toml::Value>) -> Result<Self, Self::Error> {
        Ok(Configuration {
            default_language: match value.get("default-language") {
                Some(Value::String(lang)) => Some(lang.to_owned()),
                None => None,
                _ => {
                    log::error!("field `default-language` has invalid data type (expected string)");
                    None
                }
            },
        })
    }
}
