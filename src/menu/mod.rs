pub mod menu_bulletin;
pub mod menu_main;
// pub mod user_menu;
// pub mod message_menu;
// pub mod file_menu;

use crate::box_renderer::MenuItem;
use crate::config::BbsConfig;
use crate::errors::{BbsError, BbsResult};
use crossterm::style::Color;

/// Current menu state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurrentMenu {
    Main,
    Bulletins,
    // Users,
    // Messages,
    // Files,
}

/// Actions that menus can return
#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    Stay,
    GoTo(CurrentMenu),
    Login,
    Logout,
    Quit,
    ShowMessage(String),
}

/// Lightweight data struct passed to menus - contains only what they need to read
#[derive(Copy, Clone)]
pub struct MenuData<'a> {
    pub config: &'a BbsConfig,
    pub username: &'a Option<String>,
}

impl<'a> MenuData<'a> {
    /// Helper to check if user is logged in
    pub fn is_logged_in(&self) -> bool {
        self.username.is_some()
    }

    /// Helper to check if anonymous access is allowed
    pub fn allow_anonymous(&self) -> bool {
        self.config.features.allow_anonymous
    }

    /// Get the current username, or "Anonymous" if not logged in
    pub fn display_username(&self) -> String {
        match self.username {
            Some(name) => name.clone(),
            None => "Anonymous".to_string(),
        }
    }
}

/// Output from menu rendering - contains all display information
#[derive(Debug)]
pub struct MenuRender {
    pub title: String,
    pub items: Vec<MenuItem>,
    pub prompt: String,
}

impl MenuRender {
    pub fn new(title: &str, prompt: &str) -> Self {
        Self {
            title: title.to_string(),
            items: Vec::new(),
            prompt: prompt.to_string(),
        }
    }

    pub fn with_items(title: &str, items: Vec<MenuItem>, prompt: &str) -> Self {
        Self {
            title: title.to_string(),
            items,
            prompt: prompt.to_string(),
        }
    }
}

/// Display interface that session provides to show menu output
pub trait Display {
    fn show_menu(&mut self, render: &MenuRender) -> BbsResult<()>;
    fn show_message(&mut self, title: &str, message: &str, color: Option<Color>) -> BbsResult<()>;
    fn show_feature_disabled(&mut self, feature_name: &str, sysop_name: &str) -> BbsResult<()>;
}

/// The Menu trait - clean interface with no I/O dependencies
pub trait Menu {
    /// Render the menu - pure function that returns display data
    fn render(&self, data: MenuData) -> MenuRender;

    /// Handle user input - pure function that returns an action
    fn handle_input(&self, data: MenuData, input: &str) -> MenuAction;

    /// Optional method with default implementation for menu name
    fn name(&self) -> &'static str {
        "Menu"
    }
}
