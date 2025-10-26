use crate::box_renderer::BoxStyle;
use crate::errors::ConfigError;

use std::fs;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum WidthMode {
    Auto,
    Fixed,
}

#[derive(Debug, Clone)]
pub enum AutoDetectOption {
    Auto,
    Enabled,
    Disabled,
}

#[derive(Debug, Clone)]
pub struct BbsConfig {
    pub server: ServerConfig,
    pub bbs: BbsInfo,
    pub timeouts: TimeoutConfig,
    pub features: FeatureConfig,
    pub ui: UIConfig,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub telnet_port: u16,
    pub ssh_port: Option<u16>,
    pub bind_address: String,
    pub max_connections: usize,
}

#[derive(Debug, Clone)]
pub struct BbsInfo {
    pub name: String,
    pub tagline: String,
    pub sysop_name: String,
    pub location: String,
    pub established: String,
}

#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub login_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct FeatureConfig {
    pub allow_anonymous: bool,
    pub require_registration: bool,
    pub max_message_length: usize,
    pub max_username_length: usize,
    pub file_uploads_enabled: bool,
    pub bulletins_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct UIConfig {
    pub box_style: BoxStyle,
    pub use_colors: bool,
    pub welcome_pause_ms: u64,
    // Phase 7: Clean width configuration
    pub width_mode: WidthMode,
    pub width_value: usize,
    pub ansi_support: AutoDetectOption,
    pub color_support: AutoDetectOption,
    pub adaptive_layout: bool,
}

impl Default for BbsConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                telnet_port: 2323,
                ssh_port: None,
                bind_address: "127.0.0.1".to_string(),
                max_connections: 50,
            },
            bbs: BbsInfo {
                name: "Rust BBS".to_string(),
                tagline: "A nostalgic bulletin board system built in Rust".to_string(),
                sysop_name: "SysOp".to_string(),
                location: "Cyberspace".to_string(),
                established: "2025".to_string(),
            },
            timeouts: TimeoutConfig {
                connection_timeout: Duration::from_secs(300), // 5 minutes
                idle_timeout: Duration::from_secs(1800),      // 30 minutes
                login_timeout: Duration::from_secs(120),      // 2 minutes
            },
            features: FeatureConfig {
                allow_anonymous: true,
                require_registration: false,
                max_message_length: 4096,
                max_username_length: 20,
                file_uploads_enabled: true,
                bulletins_enabled: true,
            },
            ui: UIConfig {
                box_style: BoxStyle::Ascii,
                use_colors: false,
                welcome_pause_ms: 1500,
                // Phase 7: Clean width configuration
                width_mode: WidthMode::Auto,
                width_value: 80,
                ansi_support: AutoDetectOption::Auto,
                color_support: AutoDetectOption::Auto,
                adaptive_layout: true,
            },
        }
    }
}

impl BbsConfig {
    pub fn load_from_file(path: &str) -> Result<Self, ConfigError> {
        match fs::read_to_string(path) {
            Ok(content) => Self::parse_config(&content),
            Err(_) => {
                // Create default config file if it doesn't exist
                let default_config = Self::default();
                let config_content = default_config.to_config_file_format();
                if let Err(e) = fs::write(path, config_content) {
                    eprintln!("Warning: Could not create default config file: {}", e);
                }
                Ok(default_config)
            }
        }
    }

    fn parse_config(content: &str) -> Result<Self, ConfigError> {
        let mut config = Self::default();
        let mut current_section = String::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Handle sections
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len() - 1].to_string();
                continue;
            }

            // Handle key-value pairs
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let value = line[eq_pos + 1..].trim().trim_matches('"');

                match current_section.as_str() {
                    "server" => config.parse_server_config(key, value)?,
                    "bbs" => config.parse_bbs_config(key, value)?,
                    "timeouts" => config.parse_timeout_config(key, value)?,
                    "features" => config.parse_feature_config(key, value)?,
                    "ui" => config.parse_ui_config(key, value)?,
                    _ => return Err(ConfigError::UnknownSection(current_section.clone())),
                }
            }
        }

        Ok(config)
    }

    fn parse_server_config(&mut self, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            "telnet_port" => {
                self.server.telnet_port = value
                    .parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            }
            "ssh_port" => {
                if value.is_empty() || value == "none" {
                    self.server.ssh_port = None;
                } else {
                    self.server.ssh_port = Some(value.parse().map_err(|_| {
                        ConfigError::InvalidValue(key.to_string(), value.to_string())
                    })?);
                }
            }
            "bind_address" => {
                self.server.bind_address = value.to_string();
            }
            "max_connections" => {
                self.server.max_connections = value
                    .parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            }
            _ => return Err(ConfigError::UnknownKey(key.to_string())),
        }
        Ok(())
    }

    fn parse_ui_config(&mut self, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            "box_style" => {
                self.ui.box_style = BoxStyle::from_str(value)
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            }
            "width_mode" => {
                self.ui.width_mode = match value {
                    "auto" => WidthMode::Auto,
                    "fixed" => WidthMode::Fixed,
                    _ => {
                        return Err(ConfigError::InvalidValue(
                            key.to_string(),
                            value.to_string(),
                        ));
                    }
                };
            }
            "width_value" => {
                self.ui.width_value = value
                    .parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            }
            "use_colors" => {
                self.ui.use_colors = value
                    .parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            }
            "welcome_pause_ms" => {
                self.ui.welcome_pause_ms = value
                    .parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            }

            "ansi_support" => {
                self.ui.ansi_support = match value {
                    "auto" => AutoDetectOption::Auto,
                    "true" => AutoDetectOption::Enabled,
                    "false" => AutoDetectOption::Disabled,
                    _ => {
                        return Err(ConfigError::InvalidValue(
                            key.to_string(),
                            value.to_string(),
                        ));
                    }
                };
            }
            "color_support" => {
                self.ui.color_support = match value {
                    "auto" => AutoDetectOption::Auto,
                    "true" => AutoDetectOption::Enabled,
                    "false" => AutoDetectOption::Disabled,
                    _ => {
                        return Err(ConfigError::InvalidValue(
                            key.to_string(),
                            value.to_string(),
                        ));
                    }
                };
            }
            "adaptive_layout" => {
                self.ui.adaptive_layout = value
                    .parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            }

            _ => return Err(ConfigError::UnknownKey(key.to_string())),
        }
        Ok(())
    }

    fn parse_bbs_config(&mut self, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            "name" => self.bbs.name = value.to_string(),
            "tagline" => self.bbs.tagline = value.to_string(),
            "sysop_name" => self.bbs.sysop_name = value.to_string(),
            "location" => self.bbs.location = value.to_string(),
            "established" => self.bbs.established = value.to_string(),
            _ => return Err(ConfigError::UnknownKey(key.to_string())),
        }
        Ok(())
    }

    fn parse_timeout_config(&mut self, key: &str, value: &str) -> Result<(), ConfigError> {
        let seconds: u64 = value
            .parse()
            .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;

        match key {
            "connection_timeout" => self.timeouts.connection_timeout = Duration::from_secs(seconds),
            "idle_timeout" => self.timeouts.idle_timeout = Duration::from_secs(seconds),
            "login_timeout" => self.timeouts.login_timeout = Duration::from_secs(seconds),
            _ => return Err(ConfigError::UnknownKey(key.to_string())),
        }
        Ok(())
    }

    fn parse_feature_config(&mut self, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            "allow_anonymous" => {
                self.features.allow_anonymous = value
                    .parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            }
            "require_registration" => {
                self.features.require_registration = value
                    .parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            }
            "max_message_length" => {
                self.features.max_message_length = value
                    .parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            }
            "max_username_length" => {
                self.features.max_username_length = value
                    .parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            }
            "file_uploads_enabled" => {
                self.features.file_uploads_enabled = value
                    .parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            }
            "bulletins_enabled" => {
                self.features.bulletins_enabled = value
                    .parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            }
            _ => return Err(ConfigError::UnknownKey(key.to_string())),
        }
        Ok(())
    }

    fn to_config_file_format(&self) -> String {
        format!(
            r#"# Rust BBS Configuration File
# Lines starting with # are comments

[server]
# Network configuration
telnet_port = {}
ssh_port = {}
bind_address = "{}"
max_connections = {}

[bbs]
# BBS identification and branding
name = "{}"
tagline = "{}"
sysop_name = "{}"
location = "{}"
established = "{}"

[timeouts]
# Timeout values in seconds
connection_timeout = {}
idle_timeout = {}
login_timeout = {}

[features]
# Feature toggles
allow_anonymous = {}
require_registration = {}
max_message_length = {}
max_username_length = {}
file_uploads_enabled = {}
bulletins_enabled = {}

[ui]
# User interface configuration
# Box styles: "ascii" (telnet-safe), "single", "double", "rounded"  
# Use "ascii" for best telnet compatibility
box_style = "{}"
use_colors = {}
welcome_pause_ms = {}

# Phase 7: Terminal width configuration
width_mode = "{}"          # "auto" or "fixed"
width_value = {}           # Width in characters (fixed value or fallback for auto)

# Phase 7: Terminal capability detection
ansi_support = "{}"        # "auto", "true", "false"  
color_support = "{}"       # "auto", "true", "false"
adaptive_layout = {}       # Enable responsive design
"#,
            self.server.telnet_port,
            self.server
                .ssh_port
                .map_or("none".to_string(), |p| p.to_string()),
            self.server.bind_address,
            self.server.max_connections,
            self.bbs.name,
            self.bbs.tagline,
            self.bbs.sysop_name,
            self.bbs.location,
            self.bbs.established,
            self.timeouts.connection_timeout.as_secs(),
            self.timeouts.idle_timeout.as_secs(),
            self.timeouts.login_timeout.as_secs(),
            self.features.allow_anonymous,
            self.features.require_registration,
            self.features.max_message_length,
            self.features.max_username_length,
            self.features.file_uploads_enabled,
            self.features.bulletins_enabled,
            match self.ui.box_style {
                // BoxStyleName::Double => "double",
                // BoxStyleName::Single => "single",
                // BoxStyleName::Rounded => "rounded",
                BoxStyle::Ascii => "ascii",
            },
            self.ui.use_colors,
            self.ui.welcome_pause_ms,
            match &self.ui.width_mode {
                WidthMode::Auto => "auto",
                WidthMode::Fixed => "fixed",
            },
            self.ui.width_value,
            match &self.ui.ansi_support {
                AutoDetectOption::Auto => "auto",
                AutoDetectOption::Enabled => "true",
                AutoDetectOption::Disabled => "false",
            },
            match &self.ui.color_support {
                AutoDetectOption::Auto => "auto",
                AutoDetectOption::Enabled => "true",
                AutoDetectOption::Disabled => "false",
            },
            self.ui.adaptive_layout,
        )
    }
}
