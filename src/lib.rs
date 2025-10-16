use anyhow::Result;
use dprint_core::configuration::NewLineKind;
use dprint_core::configuration::RECOMMENDED_GLOBAL_CONFIGURATION;
use dprint_core::configuration::get_unknown_property_diagnostics;
use dprint_core::configuration::get_value;
use dprint_core::configuration::resolve_new_line_kind;
use dprint_core::configuration::{ConfigKeyMap, ConfigKeyValue, GlobalConfiguration};
use dprint_core::plugins::CheckConfigUpdatesMessage;
use dprint_core::plugins::ConfigChange;
use dprint_core::plugins::FormatResult;
use dprint_core::plugins::PluginInfo;
use dprint_core::plugins::PluginResolveConfigurationResult;
use dprint_core::plugins::SyncFormatRequest;
use dprint_core::plugins::SyncHostFormatRequest;
use dprint_core::plugins::SyncPluginHandler;
use serde::{Deserialize, Serialize};
use sqlformat::FormatOptions;
use sqlformat::Indent;
use sqlformat::QueryParams;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
    pub use_tabs: bool,
    pub indent_width: u8,
    pub new_line_kind: NewLineKind,
    pub uppercase: bool,
    pub lines_between_queries: u8,
}

#[derive(Default)]
pub struct ConfigurationBuilder {
    config: ConfigKeyMap,
    global_config: Option<GlobalConfiguration>,
}

impl ConfigurationBuilder {
    /// Constructs a new configuration builder.
    pub fn new() -> ConfigurationBuilder {
        Self::default()
    }

    /// Gets the final configuration that can be used to format a file.
    pub fn build(&self) -> Configuration {
        if let Some(global_config) = &self.global_config {
            resolve_config(self.config.clone(), global_config).config
        } else {
            resolve_config(self.config.clone(), &Default::default()).config
        }
    }

    /// Set the global configuration.
    pub fn global_config(&mut self, global_config: GlobalConfiguration) -> &mut Self {
        self.global_config = Some(global_config);
        self
    }

    /// Whether to use tabs (true) or spaces (false).
    ///
    /// Default: `false`
    pub fn use_tabs(&mut self, value: bool) -> &mut Self {
        self.insert("useTabs", value.into())
    }

    /// The number of columns for an indent.
    ///
    /// Default: `4`
    pub fn indent_width(&mut self, value: u8) -> &mut Self {
        self.insert("indentWidth", (value as i32).into())
    }

    /// The kind of newline to use.
    /// Default: `NewLineKind::LineFeed`
    pub fn new_line_kind(&mut self, value: NewLineKind) -> &mut Self {
        self.insert("newLineKind", value.to_string().into())
    }

    /// Use ALL CAPS for reserved words.
    /// Default: `false`
    pub fn uppercase(&mut self, value: bool) -> &mut Self {
        self.insert("uppercase", value.into())
    }

    /// Number of line breaks between queries.
    /// Default: `1`
    pub fn lines_between_queries(&mut self, value: u8) -> &mut Self {
        self.insert("linesBetweenQueries", (value as i32).into())
    }

    #[cfg(test)]
    fn get_inner_config(&self) -> ConfigKeyMap {
        self.config.clone()
    }

    fn insert(&mut self, name: &str, value: ConfigKeyValue) -> &mut Self {
        self.config.insert(String::from(name), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use dprint_core::configuration::{NewLineKind, resolve_global_config};

    use super::*;

    #[test]
    fn check_all_values_set() {
        let mut config = ConfigurationBuilder::new();
        config
            .new_line_kind(NewLineKind::CarriageReturnLineFeed)
            .use_tabs(true)
            .indent_width(4)
            .uppercase(true)
            .lines_between_queries(2);

        let inner_config = config.get_inner_config();
        assert_eq!(inner_config.len(), 5);
        let diagnostics = resolve_config(
            inner_config,
            &resolve_global_config(&mut Default::default()).config,
        )
        .diagnostics;
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn handle_global_config() {
        let mut global_config = ConfigKeyMap::new();
        global_config.insert(String::from("newLineKind"), "crlf".into());
        global_config.insert(String::from("useTabs"), true.into());
        let global_config = resolve_global_config(&mut global_config).config;
        let mut config_builder = ConfigurationBuilder::new();
        let config = config_builder.global_config(global_config).build();
        assert_eq!(
            config.new_line_kind == NewLineKind::CarriageReturnLineFeed,
            true
        );
        assert_eq!(config.use_tabs, true);
    }

    #[test]
    fn use_defaults_when_global_not_set() {
        let global_config = resolve_global_config(&mut Default::default()).config;
        let mut config_builder = ConfigurationBuilder::new();
        let config = config_builder.global_config(global_config).build();
        assert_eq!(config.indent_width, 2);
        assert_eq!(config.new_line_kind == NewLineKind::LineFeed, true);
    }
}

pub fn resolve_config(
    config: ConfigKeyMap,
    global_config: &GlobalConfiguration,
) -> PluginResolveConfigurationResult<Configuration> {
    let mut diagnostics = Vec::new();
    let mut config = config;

    let resolved_config = Configuration {
        use_tabs: get_value(
            &mut config,
            "useTabs",
            global_config
                .use_tabs
                .unwrap_or(RECOMMENDED_GLOBAL_CONFIGURATION.use_tabs),
            &mut diagnostics,
        ),
        indent_width: get_value(
            &mut config,
            "indentWidth",
            global_config
                .indent_width
                .unwrap_or(RECOMMENDED_GLOBAL_CONFIGURATION.indent_width),
            &mut diagnostics,
        ),
        new_line_kind: get_value(
            &mut config,
            "newLineKind",
            global_config
                .new_line_kind
                .unwrap_or(RECOMMENDED_GLOBAL_CONFIGURATION.new_line_kind),
            &mut diagnostics,
        ),
        uppercase: get_value(&mut config, "uppercase", false, &mut diagnostics),
        lines_between_queries: get_value(&mut config, "linesBetweenQueries", 1, &mut diagnostics),
    };

    diagnostics.extend(get_unknown_property_diagnostics(config));

    PluginResolveConfigurationResult {
        config: resolved_config,
        diagnostics,
        file_matching: dprint_core::plugins::FileMatchingInfo {
            file_extensions: vec!["sql".to_string()],
            file_names: vec![],
        },
    }
}

pub fn format_text(text: &str, config: &Configuration) -> Result<Option<String>> {
    let input_text = text;
    let text = sqlformat::format(
        text,
        &QueryParams::None,
        &FormatOptions {
            indent: if config.use_tabs {
                Indent::Tabs
            } else {
                Indent::Spaces(config.indent_width)
            },
            uppercase: Some(config.uppercase),
            lines_between_queries: config.lines_between_queries,
            ..Default::default()
        },
    );

    // ensure ends with newline
    let text = if !text.ends_with('\n') {
        let mut text = text;
        text.push('\n');
        text
    } else {
        text
    };

    // newline
    let text = if resolve_new_line_kind(&text, config.new_line_kind) == "\n" {
        text.replace("\r\n", "\n")
    } else {
        // lazy
        text.replace("\r\n", "\n").replace("\n", "\r\n")
    };

    if text == input_text {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}

struct SqlPluginHandler {}

impl SqlPluginHandler {
    #[allow(dead_code)]
    pub const fn new() -> Self {
        Self {}
    }
}

impl SyncPluginHandler<Configuration> for SqlPluginHandler {
    fn resolve_config(
        &mut self,
        config: ConfigKeyMap,
        global_config: &GlobalConfiguration,
    ) -> PluginResolveConfigurationResult<Configuration> {
        resolve_config(config, global_config)
    }

    fn check_config_updates(
        &self,
        _message: CheckConfigUpdatesMessage,
    ) -> Result<Vec<ConfigChange>, anyhow::Error> {
        Ok(Vec::new())
    }

    fn plugin_info(&mut self) -> PluginInfo {
        let version = env!("CARGO_PKG_VERSION").to_string();
        PluginInfo {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: version.clone(),
            config_key: "sql".to_string(),
            help_url: "https://dprint.dev/plugins/sql".to_string(),
            config_schema_url: format!(
                "https://plugins.dprint.dev/dprint/dprint-plugin-sql/{}/schema.json",
                version
            ),
            update_url: Some(
                "https://plugins.dprint.dev/dprint/dprint-plugin-sql/latest.json".to_string(),
            ),
        }
    }

    fn license_text(&mut self) -> String {
        std::str::from_utf8(include_bytes!("../LICENSE"))
            .unwrap()
            .into()
    }

    fn format(
        &mut self,
        request: SyncFormatRequest<Configuration>,
        mut _format_with_host: impl FnMut(SyncHostFormatRequest) -> FormatResult,
    ) -> FormatResult {
        let file_text = String::from_utf8(request.file_bytes)?;
        format_text(&file_text, request.config).map(|maybe_text| maybe_text.map(|t| t.into_bytes()))
    }
}

#[cfg(target_arch = "wasm32")]
use dprint_core::generate_plugin_code;
#[cfg(target_arch = "wasm32")]
dprint_core::generate_plugin_code!(SqlPluginHandler, SqlPluginHandler::new());
