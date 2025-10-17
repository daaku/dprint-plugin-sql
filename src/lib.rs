use anyhow::Result;
use dprint_core::configuration::NewLineKind;
use dprint_core::configuration::RECOMMENDED_GLOBAL_CONFIGURATION;
use dprint_core::configuration::get_unknown_property_diagnostics;
use dprint_core::configuration::resolve_new_line_kind;
use dprint_core::configuration::{ConfigKeyMap, GlobalConfiguration};
use dprint_core::configuration::{get_nullable_value, get_value};
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
    pub inline: bool,
    pub max_inline_block: usize,
    pub max_inline_arguments: Option<usize>,
    pub max_inline_top_level: Option<usize>,
    pub joins_as_top_level: bool,
}

impl From<&Configuration> for FormatOptions<'_> {
    fn from(config: &Configuration) -> Self {
        FormatOptions {
            indent: if config.use_tabs {
                Indent::Tabs
            } else {
                Indent::Spaces(config.indent_width)
            },
            uppercase: Some(config.uppercase),
            lines_between_queries: config.lines_between_queries,
            inline: config.inline,
            max_inline_block: config.max_inline_block,
            max_inline_arguments: config.max_inline_arguments,
            max_inline_top_level: config.max_inline_top_level,
            joins_as_top_level: config.joins_as_top_level,
            ..Default::default()
        }
    }
}

impl Default for Configuration {
    fn default() -> Self {
        SqlPluginHandler::new()
            .resolve_config(Default::default(), &Default::default())
            .config
    }
}

pub fn format_text(text: &str, config: &Configuration) -> Result<Option<String>> {
    let input_text = text;
    let text = sqlformat::format(text, &QueryParams::None, &config.into());

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

pub struct SqlPluginHandler {}

impl SqlPluginHandler {
    #[allow(dead_code, clippy::new_without_default)]
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
        let mut diagnostics = Vec::new();
        let mut config = config;
        let default_format_options = FormatOptions::default();

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
            lines_between_queries: get_value(
                &mut config,
                "linesBetweenQueries",
                default_format_options.lines_between_queries,
                &mut diagnostics,
            ),
            inline: get_value(
                &mut config,
                "inline",
                default_format_options.inline,
                &mut diagnostics,
            ),
            max_inline_block: get_value(
                &mut config,
                "maxInlineBlock",
                default_format_options.max_inline_block,
                &mut diagnostics,
            ),
            max_inline_arguments: get_nullable_value(
                &mut config,
                "maxInlineArguments",
                &mut diagnostics,
            ),
            max_inline_top_level: get_nullable_value(
                &mut config,
                "maxInlineTopLevel",
                &mut diagnostics,
            ),
            joins_as_top_level: get_value(
                &mut config,
                "joinsAsTopLevel",
                default_format_options.joins_as_top_level,
                &mut diagnostics,
            ),
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
            help_url: "https://github.com/daaku/dprint-plugin-sql".to_string(),
            config_schema_url: format!(
                "https://plugins.dprint.dev/daaku/sql/{}/schema.json",
                version
            ),
            update_url: Some("https://plugins.dprint.dev/daaku/sql/latest.json".to_string()),
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
