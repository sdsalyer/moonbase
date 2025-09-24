use super::{Menu, MenuAction, MenuRender, MenuScreen};
use crate::{box_renderer::MenuItem, session::BbsSession};

/// User menu - can have state like current user view, sort order, etc.
pub struct UserMenu {
    sort_by_last_login: bool,
    show_offline_users: bool,
}

impl UserMenu {
    pub fn new() -> Self {
        Self {
            sort_by_last_login: false,
            show_offline_users: true,
        }
    }
}

impl MenuScreen for UserMenu {
    // fn name(&self) -> &'static str {
    //     "User Directory"
    // }

    fn render(&self, data: &BbsSession) -> MenuRender {
        let mut items = vec![];

        match &data.user_stats {
            Some(stats) => {
                // Real user statistics from storage
                items.push(MenuItem::info(&format!(
                    "👥 Users currently online: {}",
                    stats.online_users
                )));
                items.push(MenuItem::info(&format!(
                    "👤 Total registered users: {}",
                    stats.total_users
                )));
                items.push(MenuItem::separator());

                // Show sort/filter status
                let sort_info = if self.sort_by_last_login {
                    "Sorted by: Last login"
                } else {
                    "Sorted by: Username"
                };
                items.push(MenuItem::info(sort_info));

                if !self.show_offline_users {
                    items.push(MenuItem::info("(Hiding offline users)"));
                }

                items.push(MenuItem::separator());
                items.push(MenuItem::info("Recent logins:"));

                // Display real recent login data
                for login in &stats.recent_logins {
                    let suffix = if login.is_current_user { " (you)" } else { "" };
                    items.push(MenuItem::info(&format!(
                        "* {} - {}{}",
                        login.username, login.last_login_display, suffix
                    )));
                }

                items.push(MenuItem::separator());
            }

            None => {}
        }

        items.push(MenuItem::option("L", "List all users"));
        items.push(MenuItem::option("W", "Who's online"));

        // Stateful options
        if self.sort_by_last_login {
            items.push(MenuItem::option("N", "Sort by username"));
        } else {
            items.push(MenuItem::option("T", "Sort by last login"));
        }

        if data.is_logged_in() {
            items.push(MenuItem::option("P", "View your profile"));
        }

        items.push(MenuItem::option("B", "Back to main"));

        MenuRender::with_items("USER DIRECTORY", items, "\nChoice: ")
    }

    fn handle_input(&self, data: &BbsSession, input: &str) -> MenuAction {
        match input.to_lowercase().as_str() {
            "l" => {
                // Build real online users list
                let mut total_msg = "Users currently registered:\n".to_string();

                match &data.user_stats {
                    Some(stats) => {
                        if stats.total_users > 0 {
                            total_msg.push_str(&format!(
                                "* {} total users currently registered",
                                stats.total_users
                            ));
                            for username in stats.all_users.iter() {
                                total_msg.push_str(&format!("\n > {}", username));
                            }
                        } else {
                            total_msg.push_str("* No users currently registered");
                        }
                    }
                    None => {
                        // For now, show current user if logged in, or anonymous
                        if data.is_logged_in() {
                            total_msg.push_str(&format!("* {} (you)", data.display_username()));
                        } else {
                            // total_msg.push_str("* Anonymous (you)");
                            total_msg.push_str("* No users currently registered");
                        }
                    }
                }

                MenuAction::ShowMessage(total_msg)
            }
            "w" => {
                // Build real online users list
                let mut online_msg = "Users currently online:\n".to_string();

                match &data.user_stats {
                    Some(stats) => {
                        // TODO: list all online users here?
                        if stats.online_users > 1 {
                            online_msg.push_str(&format!(
                                "\n* ... and {} others",
                                stats.online_users - 1
                            ));
                        } else {
                            online_msg.push_str("* No users currently online");
                        }
                    }
                    None => {
                        // For now, show current user if logged in, or anonymous
                        if data.is_logged_in() {
                            online_msg.push_str(&format!("* {} (you)", data.display_username()));
                        } else {
                            online_msg.push_str("* Anonymous (you)");
                        }
                    }
                }

                MenuAction::ShowMessage(online_msg)
            }
            "n" | "t" => {
                MenuAction::ShowMessage("Sort order changed (would toggle sort)".to_string())
            }
            "p" => {
                if data.is_logged_in() {
                    let profile_msg = format!(
                        "User Profile:\nUsername: {}\nJoined: Today\nLast Login: Now\n\n(Full profile features coming soon!)",
                        data.display_username()
                    );
                    MenuAction::ShowMessage(profile_msg)
                } else {
                    MenuAction::ShowMessage("Invalid choice.".to_string())
                }
            }
            "b" => MenuAction::GoTo(Menu::Main),
            _ => {
                if data.is_logged_in() {
                    MenuAction::ShowMessage("Invalid choice. Use L, W, N/T, P, or B.".to_string())
                } else {
                    MenuAction::ShowMessage("Invalid choice. Use L, W, N/T, or B.".to_string())
                }
            }
        }
    }
}
