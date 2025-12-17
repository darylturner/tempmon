use std::collections::HashMap;
use std::fs;

use serde::Deserialize;

const CONFIG_PATH: &str = "/etc/tempmon/config.toml";

#[derive(Debug, Deserialize)]
pub struct Config {
    pub settings: Settings,
    pub probe_labels: HashMap<String, String>,
    #[serde(default)]
    pub calibration_offsets: HashMap<String, f32>,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub metrics_port: u16,
    pub probe_interval: u64,
    pub probe_resolution: u8,
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(CONFIG_PATH)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_config() {
        let toml_str = r#"
[settings]
metrics_port = 9184
probe_interval = 15
probe_resolution = 10

[probe_labels]
"28-abc123" = "test_probe"
"28-def456" = "another_probe"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.settings.metrics_port, 9184);
        assert_eq!(config.settings.probe_interval, 15);
        assert_eq!(config.settings.probe_resolution, 10);
        assert_eq!(
            config.probe_labels.get("28-abc123"),
            Some(&"test_probe".to_string())
        );
        assert_eq!(
            config.probe_labels.get("28-def456"),
            Some(&"another_probe".to_string())
        );
    }

    #[test]
    fn test_parse_config_with_no_labels() {
        let toml_str = r#"
[settings]
metrics_port = 9184
probe_interval = 30
probe_resolution = 12

[probe_labels]
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.settings.metrics_port, 9184);
        assert_eq!(config.settings.probe_interval, 30);
        assert_eq!(config.settings.probe_resolution, 12);
        assert!(config.probe_labels.is_empty());
    }

    #[test]
    fn test_parse_config_minimal() {
        let toml_str = r#"
[settings]
metrics_port = 8080
probe_interval = 5
probe_resolution = 9

[probe_labels]
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.settings.metrics_port, 8080);
        assert_eq!(config.settings.probe_interval, 5);
        assert_eq!(config.settings.probe_resolution, 9);
    }

    #[test]
    fn test_parse_config_with_calibration_offsets() {
        let toml_str = r#"
[settings]
metrics_port = 9184
probe_interval = 15
probe_resolution = 10

[probe_labels]
"28-abc123" = "test_probe"

[calibration_offsets]
"28-abc123" = 0.5
"28-def456" = -0.3
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.calibration_offsets.get("28-abc123"), Some(&0.5));
        assert_eq!(config.calibration_offsets.get("28-def456"), Some(&-0.3));
    }

    #[test]
    fn test_parse_config_without_calibration_offsets() {
        // Test backwards compatibility - should work without [calibration_offsets] section
        let toml_str = r#"
[settings]
metrics_port = 9184
probe_interval = 15
probe_resolution = 10

[probe_labels]
"28-abc123" = "test_probe"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.calibration_offsets.is_empty());
    }

    #[test]
    fn test_parse_config_with_empty_calibration_offsets() {
        let toml_str = r#"
[settings]
metrics_port = 9184
probe_interval = 15
probe_resolution = 10

[probe_labels]

[calibration_offsets]
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.calibration_offsets.is_empty());
    }
}
