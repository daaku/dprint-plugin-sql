pub mod configuration;
mod format_text;

use configuration::{Configuration, resolve_config};
use dprint_core::configuration::{ConfigKeyMap, GlobalConfiguration};
use dprint_core::plugins::CheckConfigUpdatesMessage;
use dprint_core::plugins::ConfigChange;
use dprint_core::plugins::FormatResult;
use dprint_core::plugins::PluginInfo;
use dprint_core::plugins::PluginResolveConfigurationResult;
use dprint_core::plugins::SyncFormatRequest;
use dprint_core::plugins::SyncHostFormatRequest;
use dprint_core::plugins::SyncPluginHandler;
pub use format_text::format_text;

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

  fn check_config_updates(&self, _message: CheckConfigUpdatesMessage) -> Result<Vec<ConfigChange>, anyhow::Error> {
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
      update_url: Some("https://plugins.dprint.dev/dprint/dprint-plugin-sql/latest.json".to_string()),
    }
  }

  fn license_text(&mut self) -> String {
    std::str::from_utf8(include_bytes!("../LICENSE")).unwrap().into()
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
