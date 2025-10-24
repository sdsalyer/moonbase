use super::{Menu, MenuAction, MenuRender, MenuScreen};
use crate::{box_renderer::MenuItem, session::BbsSession};

/// Main menu - clean, stateless menu
pub struct MainMenu;

impl MainMenu {
    pub fn new() -> Self {
        Self
    }
}

impl MenuScreen for MainMenu {
    // fn name(&self) -> &'static str {
    //     "Main Menu"
    // }

    fn render(&self, data: &BbsSession) -> MenuRender {
        let title = format!("{} - MAIN MENU", data.config.bbs.name);
        let mut items = vec![];

        // User status
        let user_status = if data.is_logged_in() {
            format!("Logged in as: {}", data.display_username())
        } else if data.allow_anonymous() {
            "Status: Anonymous User".to_string()
        } else {
            "Status: Guest (Limited Access)".to_string()
        };

        items.push(MenuItem::info(&user_status));
        items.push(MenuItem::separator());

        // Menu options based on config
        if data.config.features.bulletins_enabled {
            items.push(MenuItem::option("1", "Bulletin Board"));
        } else {
            items.push(MenuItem::disabled_option("1", "Bulletin Board"));
        }

        items.push(MenuItem::option("2", "User Directory"));
        
        // Private Messages with unread count
        if data.is_logged_in() {
            if let Some(user) = &data.user {
                if let Ok(unread_count) = data.services.messages.get_unread_count(&user.username) {
                    if unread_count > 0 {
                        items.push(MenuItem::option("3", &format!("Private Messages [{}]", unread_count)));
                    } else {
                        items.push(MenuItem::option("3", "Private Messages"));
                    }
                } else {
                    items.push(MenuItem::option("3", "Private Messages"));
                }
            } else {
                items.push(MenuItem::option("3", "Private Messages"));
            }
        } else {
            items.push(MenuItem::option("3", "Private Messages"));
        }

        if data.config.features.file_uploads_enabled {
            items.push(MenuItem::option("4", "File Library"));
        } else {
            items.push(MenuItem::disabled_option("4", "File Library"));
        }

        items.push(MenuItem::separator());

        // Login/logout options
        if !data.is_logged_in() && data.allow_anonymous() {
            items.push(MenuItem::option("L", "Login / Register"));
        } else if data.is_logged_in() {
            items.push(MenuItem::option("O", "Logout"));
        }

        items.push(MenuItem::option("Q", "Quit"));

        MenuRender::with_items(&title, items, "\nEnter your choice: ")
    }

    fn handle_input(&self, data: &BbsSession, input: &str) -> MenuAction {
        match input.to_lowercase().as_str() {
            "1" => {
                if data.config.features.bulletins_enabled {
                    MenuAction::GoTo(Menu::Bulletins)
                } else {
                    MenuAction::ShowMessage("Bulletin Board is currently disabled.".to_string())
                }
            }
            "2" => MenuAction::GoTo(Menu::Users),

            "3" => {
                if data.is_logged_in() {
                    MenuAction::GoTo(Menu::Messages)
                } else {
                    MenuAction::ShowMessage("You must be logged in to use private messages.".to_string())
                }
            }
            "4" => {
                if data.config.features.file_uploads_enabled {
                    MenuAction::ShowMessage("File Library coming soon!".to_string())
                } else {
                    MenuAction::ShowMessage("File Library is currently disabled.".to_string())
                }
            }
            "l" | "login" => {
                if !data.is_logged_in() && data.allow_anonymous() {
                    MenuAction::Login
                } else if data.is_logged_in() {
                    MenuAction::ShowMessage("You are already logged in.".to_string())
                } else {
                    MenuAction::ShowMessage("Login is not available.".to_string())
                }
            }
            "o" | "logout" => {
                if data.is_logged_in() {
                    MenuAction::Logout
                } else {
                    MenuAction::ShowMessage("You are not logged in.".to_string())
                }
            }
            "q" | "quit" | "exit" => MenuAction::Quit,
            _ => MenuAction::ShowMessage("Invalid choice. Please try again.".to_string()),
        }
    }
}
