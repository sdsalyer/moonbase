use super::{CurrentMenu, Menu, MenuAction, MenuData, MenuRender};
use crate::box_renderer::MenuItem;

/// Bulletin menu - can have state like current bulletin, filters, etc.
pub struct BulletinMenu {
    // Example of menu-specific state
    // current_bulletin_id: Option<u32>,
    show_read_bulletins: bool,
}

impl BulletinMenu {
    pub fn new() -> Self {
        Self {
            // current_bulletin_id: None,
            show_read_bulletins: true,
        }
    }

    // Example of menu-specific method
    // pub fn set_filter(&mut self, show_read: bool) {
    //     self.show_read_bulletins = show_read;
    // }
}

impl Menu for BulletinMenu {
    // fn name(&self) -> &'static str {
    //     "Bulletin Board"
    // }

    fn render(&self, data: MenuData) -> MenuRender {
        if !data.config.features.bulletins_enabled {
            let items = vec![
                MenuItem::info("!  Bulletin Board has been disabled by the SysOp."),
                MenuItem::info(&format!(
                    "Contact {} for more information.",
                    data.config.bbs.sysop_name
                )),
                MenuItem::separator(),
                MenuItem::option("B", "Back to main menu"),
            ];
            return MenuRender::with_items("FEATURE DISABLED", items, "\nChoice: ");
        }

        let mut items = vec![];

        items.push(MenuItem::info("> Recent Bulletins:"));
        items.push(MenuItem::info(""));

        let welcome_msg = format!(
            "* Welcome to {}! - Posted by {}",
            data.config.bbs.name, data.config.bbs.sysop_name
        );
        items.push(MenuItem::info(&welcome_msg));
        items.push(MenuItem::info("* System maintenance tonight - SysOp"));
        items.push(MenuItem::info("* New features coming soon - Admin"));

        // Show filter status (example of using menu state)
        if !self.show_read_bulletins {
            items.push(MenuItem::info("(Hiding read bulletins)"));
        }

        items.push(MenuItem::separator());

        items.push(MenuItem::option("R", "Read bulletins"));

        // Check if user can post
        if data.is_logged_in() || data.allow_anonymous() {
            items.push(MenuItem::option("P", "Post new bulletin"));
        } else {
            items.push(MenuItem::disabled_option(
                "P",
                "Post new bulletin (login required)",
            ));
        }

        // Example of stateful menu option
        if self.show_read_bulletins {
            items.push(MenuItem::option("H", "Hide read bulletins"));
        } else {
            items.push(MenuItem::option("S", "Show read bulletins"));
        }

        items.push(MenuItem::option("B", "Back to main"));

        MenuRender::with_items("BULLETIN BOARD", items, "\nChoice: ")
    }

    fn handle_input(&self, data: MenuData, input: &str) -> MenuAction {
        if !data.config.features.bulletins_enabled {
            return match input.to_lowercase().as_str() {
                "b" => MenuAction::GoTo(CurrentMenu::Main),
                _ => MenuAction::ShowMessage(
                    "Bulletin Board is disabled. Press B to go back.".to_string(),
                ),
            };
        }

        match input.to_lowercase().as_str() {
            "r" => {
                MenuAction::ShowMessage("Reading bulletins... (Feature coming soon!)".to_string())
            }
            "p" => {
                if data.is_logged_in() || data.allow_anonymous() {
                    MenuAction::ShowMessage(
                        "Posting bulletin... (Feature coming soon!)".to_string(),
                    )
                } else {
                    MenuAction::ShowMessage("You must be logged in to post bulletins.".to_string())
                }
            }
            "h" => {
                // Note: This shows the limitation - we can't modify self with &self
                // In a real implementation, we'd need to return a different action type
                // or use interior mutability (RefCell, etc.)
                MenuAction::ShowMessage("Filter toggled (would hide read bulletins)".to_string())
            }
            "s" => {
                MenuAction::ShowMessage("Filter toggled (would show read bulletins)".to_string())
            }
            "b" => MenuAction::GoTo(CurrentMenu::Main),
            _ => MenuAction::ShowMessage("Invalid choice. Use R, P, H/S, or B.".to_string()),
        }
    }
}
