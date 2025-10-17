use daaku_dprint_plugin_sql::Configuration;
use daaku_dprint_plugin_sql::SqlPluginHandler;
use daaku_dprint_plugin_sql::format_text;
use dprint_core::configuration::ConfigKeyMap;
use dprint_core::configuration::NewLineKind;
use dprint_core::configuration::resolve_global_config;
use dprint_core::plugins::SyncPluginHandler;
use dprint_development::ParseSpecOptions;
use dprint_development::RunSpecsOptions;
use dprint_development::ensure_no_diagnostics;
use dprint_development::run_specs;
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn handle_global_config() {
    let mut global_config = ConfigKeyMap::new();
    global_config.insert(String::from("newLineKind"), "crlf".into());
    global_config.insert(String::from("useTabs"), true.into());
    let global_config = resolve_global_config(&mut global_config).config;
    let mut sph = SqlPluginHandler::new();
    let config = sph
        .resolve_config(Default::default(), &global_config)
        .config;
    assert_eq!(config.new_line_kind, NewLineKind::CarriageReturnLineFeed);
    assert_eq!(config.use_tabs, true);
}

#[test]
fn use_defaults_when_global_not_set() {
    let config = Configuration::default();
    assert_eq!(config.indent_width, 2);
    assert_eq!(config.new_line_kind, NewLineKind::LineFeed);
}

#[test]
fn test_specs() {
    let global_config = resolve_global_config(&mut Default::default()).config;
    run_specs(
        &PathBuf::from("./tests/specs"),
        &ParseSpecOptions {
            default_file_name: "file.sql",
        },
        &RunSpecsOptions {
            fix_failures: false,
            format_twice: true,
        },
        {
            let global_config = global_config.clone();
            Arc::new(move |_file_path, file_text, spec_config| {
                let spec_config: ConfigKeyMap =
                    serde_json::from_value(spec_config.clone().into()).unwrap();
                let mut sph = SqlPluginHandler::new();
                let config_result = sph.resolve_config(spec_config, &global_config);
                ensure_no_diagnostics(&config_result.diagnostics);
                format_text(file_text, &config_result.config)
            })
        },
        Arc::new(move |_file_path, _file_text, _spec_config| {
            panic!("Plugin does not support dprint-core tracing.")
        }),
    )
}

#[test]
fn should_handle_windows_newlines() {
    let config = Configuration::default();
    assert_eq!(
        format_text("SELECT * FROM  dbo.Test\r\n", &config)
            .unwrap()
            .unwrap(),
        "select\n  *\nfrom\n  dbo.Test\n",
    );
}
