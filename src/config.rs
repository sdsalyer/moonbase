use std::time::Duration;

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


#[derive(Debug)]
pub struct MoonbaseConfig {
    pub server: ServerConfig,
    pub bbs: BbsInfo,
}

#[derive(Debug)]
pub struct ServerConfig {
    pub bind_address: String,
    pub telnet_port: u16,
    pub ssh_port: Option<u16>,
    pub max_connections: usize,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub login_timeout: Duration,
}

#[derive(Debug)]
pub struct BbsInfo {
    pub name: String,
    pub sysop: String,
    pub location: String,
}

impl Default for MoonbaseConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                bind_address: "127.0.0.1".to_string(),
                telnet_port: 2323, // 0 lets the OS decide
                ssh_port: None,
                max_connections: 50,
                connection_timeout: Duration::from_secs(300),
                idle_timeout: Duration::from_secs(1800),
                login_timeout: Duration::from_secs(120),
            },
            bbs: BbsInfo {
                name: "Moonbase".to_string(),
                sysop: "SysOp".to_string(),
                location: "La Luna".to_string(),
            }
        }
    }
}

impl MoonbaseConfig {
    pub fn load_from_file(path: &str) -> Result<Self, ConfigError> {
        Err(ConfigError::IoError("TODO".to_string()))
    }

    pub fn get_welcome_banner(&self) -> String {
        todo!()
    }

    fn parse_config(conf: &str) -> Result<Self, ConfigError> {
        todo!()
    }

    fn parse_server_config(&mut self, key: &str, value: &str) -> Result<(), ConfigError> {
        todo!()
    }

    fn parse_bbs_info(&mut self, key: &str, value: &str) -> Result<(), ConfigError> {
        todo!()
    }

    fn to_config_file_format(&self) -> String {
        todo!()
    }
}
