use std::collections::HashMap;
use std::fs;
use std::time::Duration;
use crate::box_renderer::{BoxRenderer, BoxStyleName, BoxStyle};

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
    pub box_style: BoxStyleName,
    pub menu_width: usize,
    pub use_colors: bool,
    pub welcome_pause_ms: u64,
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
                connection_timeout: Duration::from_secs(300),    // 5 minutes
                idle_timeout: Duration::from_secs(1800),         // 30 minutes
                login_timeout: Duration::from_secs(120),         // 2 minutes
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
                box_style: BoxStyleName::Double,
                menu_width: 42,
                use_colors: true,
                welcome_pause_ms: 1500,
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
                current_section = line[1..line.len()-1].to_string();
                continue;
            }
            
            // Handle key-value pairs
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let value = line[eq_pos+1..].trim().trim_matches('"');
                
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
                self.server.telnet_port = value.parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            },
            "ssh_port" => {
                if value.is_empty() || value == "none" {
                    self.server.ssh_port = None;
                } else {
                    self.server.ssh_port = Some(value.parse()
                        .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?);
                }
            },
            "bind_address" => {
                self.server.bind_address = value.to_string();
            },
            "max_connections" => {
                self.server.max_connections = value.parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            },
            _ => return Err(ConfigError::UnknownKey(key.to_string())),
        }
        Ok(())
    }
    
    fn parse_ui_config(&mut self, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            "box_style" => {
                self.ui.box_style = BoxStyleName::from_str(value)
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            },
            "menu_width" => {
                self.ui.menu_width = value.parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            },
            "use_colors" => {
                self.ui.use_colors = value.parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            },
            "welcome_pause_ms" => {
                self.ui.welcome_pause_ms = value.parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            },
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
        let seconds: u64 = value.parse()
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
                self.features.allow_anonymous = value.parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            },
            "require_registration" => {
                self.features.require_registration = value.parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            },
            "max_message_length" => {
                self.features.max_message_length = value.parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            },
            "max_username_length" => {
                self.features.max_username_length = value.parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            },
            "file_uploads_enabled" => {
                self.features.file_uploads_enabled = value.parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            },
            "bulletins_enabled" => {
                self.features.bulletins_enabled = value.parse()
                    .map_err(|_| ConfigError::InvalidValue(key.to_string(), value.to_string()))?;
            },
            _ => return Err(ConfigError::UnknownKey(key.to_string())),
        }
        Ok(())
    }
    
    fn to_config_file_format(&self) -> String {
        format!(r#"# Rust BBS Configuration File
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
box_style = "{}"
menu_width = {}
use_colors = {}
welcome_pause_ms = {}
"#,
            self.server.telnet_port,
            self.server.ssh_port.map_or("none".to_string(), |p| p.to_string()),
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
                BoxStyleName::Double => "double",
                BoxStyleName::Single => "single", 
                BoxStyleName::Rounded => "rounded",
                BoxStyleName::Ascii => "ascii",
            },
            self.ui.menu_width,
            self.ui.use_colors,
            self.ui.welcome_pause_ms,
        )
    }
    
    pub fn get_welcome_header(&self) -> String {
        format!(r#"
╔══════════════════════════════════════════════════════════════════════════════╗
║                         🏛️  {}  🏛️                           ║
║                                                                              ║
║                    {}                     ║
║                        SysOp: {} • Est. {}                         ║
║                           Location: {}                            ║
╚══════════════════════════════════════════════════════════════════════════════╝
"#,
            self.bbs.name.chars().take(30).collect::<String>(),
            self.bbs.tagline.chars().take(50).collect::<String>(),
            self.bbs.sysop_name,
            self.bbs.established,
            self.bbs.location
        )
    }
}

#[derive(Debug)]
pub enum ConfigError {
    InvalidValue(String, String),
    UnknownKey(String),
    UnknownSection(String),
    IoError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidValue(key, value) => {
                write!(f, "Invalid value '{}' for key '{}'", value, key)
            },
            ConfigError::UnknownKey(key) => write!(f, "Unknown configuration key: '{}'", key),
            ConfigError::UnknownSection(section) => write!(f, "Unknown section: '{}'", section),
            ConfigError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}
