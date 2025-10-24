pub mod menu_bulletin;
pub mod menu_main;
pub mod menu_message;
pub mod menu_user;
// pub mod file_menu;

use crate::box_renderer::MenuItem;

use crate::session::BbsSession;

/// Current menu state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Menu {
    Main,
    Bulletins,
    Users,
    Messages,
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

    // TODO: generalize this like GoToSubMenu(SubMenu)?
    // Bulletin-specific actions
    BulletinPost,
    BulletinRead(u32),
    BulletinSubmit { title: String, content: String },
    BulletinPostContent(String),
    BulletinList,
    BulletinBackToMenu,
    BulletinToggleReadFilter,
    BulletinToggleUnreadOnly,

    // Message-specific actions
    MessageInbox,
    MessageSent,
    MessageCompose,
    MessageComposeSubject(String),
    MessageSend { recipient: String, subject: String, content: String },
    MessageRead(u32),
    MessageDelete(u32),
    MessageBackToMenu,
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
