//! dprint plugin wrapping [`json_schema_sort`]. It sorts JSON Schema documents
//! into a deterministic, schema-aware order so diffs stay quiet, leaving the
//! actual whitespace formatting to dprint's JSON plugin.

use dprint_core::configuration::{
    ConfigKeyMap, GlobalConfiguration, ResolveConfigurationResult,
    get_unknown_property_diagnostics, get_value,
};
use json_schema_sort::{ArraySorting, PropertyOrdering, SortOptions};
use serde::Serialize;

/// Resolved plugin configuration. Mirrors the two behavioural knobs of
/// [`json_schema_sort::SortOptions`] as dprint config keys.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
    /// Sort schema-safe string arrays such as `required` and `type`.
    pub sort_arrays: bool,
    /// Keep property and `$defs` names in document order instead of sorting.
    pub preserve_property_order: bool,
}

impl From<&Configuration> for SortOptions {
    fn from(config: &Configuration) -> Self {
        let mut options = SortOptions::default();
        options.arrays = if config.sort_arrays {
            ArraySorting::SchemaSafe
        } else {
            ArraySorting::None
        };
        options.properties = if config.preserve_property_order {
            PropertyOrdering::Preserve
        } else {
            PropertyOrdering::Sorted
        };
        options
    }
}

/// Resolve dprint config keys (`sortArrays`, `preservePropertyOrder`) into a
/// [`Configuration`], collecting diagnostics for unknown keys.
pub fn resolve_config(
    config: ConfigKeyMap,
    _global_config: &GlobalConfiguration,
) -> ResolveConfigurationResult<Configuration> {
    let mut config = config;
    let mut diagnostics = Vec::new();

    let resolved = Configuration {
        sort_arrays: get_value(&mut config, "sortArrays", true, &mut diagnostics),
        preserve_property_order: get_value(
            &mut config,
            "preservePropertyOrder",
            false,
            &mut diagnostics,
        ),
    };

    diagnostics.extend(get_unknown_property_diagnostics(config));
    ResolveConfigurationResult {
        config: resolved,
        diagnostics,
    }
}

/// Sort `text` and return `Some(sorted)` when it changes, `None` when the input
/// is already sorted.
pub fn format_text(config: &Configuration, text: &str) -> anyhow::Result<Option<String>> {
    let sorted = json_schema_sort::to_string_sorted_with_options(text, config.into())?;
    Ok((sorted != text).then_some(sorted))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(sort_arrays: bool, preserve_property_order: bool) -> Configuration {
        Configuration {
            sort_arrays,
            preserve_property_order,
        }
    }

    #[test]
    fn sorts_keys_and_reports_a_change() {
        let out = format_text(&config(true, false), r#"{"type":"object","$id":"x"}"#)
            .unwrap()
            .expect("unsorted input should produce a change");
        assert_eq!(out, "{\n  \"$id\": \"x\",\n  \"type\": \"object\"\n}\n");
    }

    #[test]
    fn already_sorted_input_reports_no_change() {
        let sorted = "{\n  \"$id\": \"x\",\n  \"type\": \"object\"\n}\n";
        assert_eq!(format_text(&config(true, false), sorted).unwrap(), None);
    }

    #[test]
    fn preserve_property_order_keeps_document_order() {
        let input = r#"{"properties":{"z":{"type":"string"},"a":{"type":"number"}}}"#;
        let out = format_text(&config(true, true), input).unwrap().unwrap();
        assert!(
            out.find("\"z\"").unwrap() < out.find("\"a\"").unwrap(),
            "property order should be preserved:\n{out}"
        );
    }
}

#[cfg(all(feature = "wasm", target_arch = "wasm32", target_os = "unknown"))]
mod wasm_plugin {
    use dprint_core::configuration::{ConfigKeyMap, GlobalConfiguration};
    use dprint_core::generate_plugin_code;
    use dprint_core::plugins::{
        CheckConfigUpdatesMessage, ConfigChange, FileMatchingInfo, FormatResult, PluginInfo,
        PluginResolveConfigurationResult, SyncFormatRequest, SyncHostFormatRequest,
        SyncPluginHandler,
    };

    use super::{Configuration, format_text, resolve_config};

    struct JsonSchemaSortPluginHandler;

    impl SyncPluginHandler<Configuration> for JsonSchemaSortPluginHandler {
        fn resolve_config(
            &mut self,
            config: ConfigKeyMap,
            global_config: &GlobalConfiguration,
        ) -> PluginResolveConfigurationResult<Configuration> {
            let result = resolve_config(config, global_config);
            PluginResolveConfigurationResult {
                config: result.config,
                diagnostics: result.diagnostics,
                file_matching: FileMatchingInfo {
                    file_extensions: vec!["json".to_string()],
                    file_names: vec![],
                },
            }
        }

        fn check_config_updates(
            &self,
            _message: CheckConfigUpdatesMessage,
        ) -> anyhow::Result<Vec<ConfigChange>> {
            Ok(Vec::new())
        }

        fn plugin_info(&mut self) -> PluginInfo {
            PluginInfo {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                config_key: "jsonSchemaSort".to_string(),
                help_url: "https://github.com/kjanat/json-schema-sort-rs".to_string(),
                config_schema_url: String::new(),
                update_url: None,
            }
        }

        fn license_text(&mut self) -> String {
            include_str!("../../../LICENSE-MIT").to_string()
        }

        fn format(
            &mut self,
            request: SyncFormatRequest<Configuration>,
            _format_with_host: impl FnMut(SyncHostFormatRequest) -> FormatResult,
        ) -> FormatResult {
            let file_text = String::from_utf8(request.file_bytes)?;
            format_text(request.config, &file_text).map(|maybe| maybe.map(String::into_bytes))
        }
    }

    generate_plugin_code!(JsonSchemaSortPluginHandler, JsonSchemaSortPluginHandler);
}
