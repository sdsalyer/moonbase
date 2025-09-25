pub mod menu_bulletin;
pub mod menu_main;
pub mod menu_user;
// pub mod message_menu;
// pub mod file_menu;

use crate::box_renderer::MenuItem;
use crate::bulletins::Bulletin;
use crate::session::BbsSession;

/// Current menu state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Menu {
    Main,
    Bulletins,
    Users,
    // Messages,
    // Files,
}

/// Actions that menus can return
#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    // Stay,
    GoTo(Menu),
    Login,
    Logout,
    Quit,
    ShowMessage(String),
}

/// Statistics about users for display in menus
#[derive(Debug, Default, Clone)]
pub struct UserStats {
    pub total_users: usize,
    pub online_users: usize,
    pub all_users: Vec<String>,
    pub recent_logins: Vec<RecentLogin>,
}

/// Information about recent user logins
#[derive(Debug, Clone)]
pub struct RecentLogin {
    pub username: String,
    pub last_login_display: String,
    pub is_current_user: bool,
}

/// Output from menu rendering - contains all display information
#[derive(Debug)]
pub struct MenuRender {
    pub title: String,
    pub items: Vec<MenuItem>,
    pub prompt: String,
}

impl MenuRender {
    // pub fn new(title: &str, prompt: &str) -> Self {
    //     Self {
    //         title: title.to_string(),
    //         items: Vec::new(),
    //         prompt: prompt.to_string(),
    //     }
    // }

    pub fn with_items(title: &str, items: Vec<MenuItem>, prompt: &str) -> Self {
        Self {
            title: title.to_string(),
            items,
            prompt: prompt.to_string(),
        }
    }
}

// Display interface that session provides to show menu output
// pub trait Display {
//     fn show_menu(&mut self, render: &MenuRender) -> BbsResult<()>;
//     fn show_message(&mut self, title: &str, message: &str, color: Option<Color>) -> BbsResult<()>;
//     fn show_feature_disabled(&mut self, feature_name: &str, sysop_name: &str) -> BbsResult<()>;
// }
//
/// The Menu trait - clean interface with no I/O dependencies
pub trait MenuScreen {
    /// Render the menu - pure function that returns display data
    fn render(&self, data: &BbsSession) -> MenuRender;

    /// Handle user input - pure function that returns an action
    fn handle_input(&self, data: &BbsSession, input: &str) -> MenuAction;

    // Optional method with default implementation for menu name
    // fn name(&self) -> &'static str {
    //     "Menu"
    // }
}

/// Bulletin statistics for display
#[derive(Debug, Clone, Default)]
pub struct BulletinStats {
    pub total_bulletins: usize,
    pub unread_count: usize,
    pub recent_bulletins: Vec<BulletinSummary>,
}

/// Summary of a bulletin for menu display
#[derive(Debug, Clone)]
pub struct BulletinSummary {
    pub id: u32,
    pub title: String,
    pub author: String,
    pub posted_display: String,
    pub is_sticky: bool,
    pub is_read: bool,
}

impl From<(&Bulletin, bool)> for BulletinSummary {
    fn from((bulletin, is_read): (&Bulletin, bool)) -> Self {
        Self {
            id: bulletin.id,
            title: bulletin.title.clone(),
            author: bulletin.author.clone(),
            posted_display: bulletin.posted_display(),
            is_sticky: bulletin.is_sticky,
            is_read,
        }
    }
}
