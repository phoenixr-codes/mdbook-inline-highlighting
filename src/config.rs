#[derive(Default)]
pub(crate) struct Configuration {
    pub default_language: Option<String>,
}

impl Configuration {
    pub(crate) fn from_mdbook_config(cfg: &mdbook_preprocessor::config::Config) -> Self {
        let default_language = cfg
            .get::<String>("preprocessor.inline-highlighting.default-language")
            .ok()
            .flatten();

        Configuration { default_language }
    }
}
