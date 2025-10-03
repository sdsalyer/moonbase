use super::{Menu, MenuAction, MenuRender, MenuScreen};
use crate::{
    box_renderer::MenuItem, bulletin_repository::BulletinStats, bulletins::Bulletin,
    session::BbsSession,
};

/// Bulletin menu actions
#[derive(Debug, Clone, PartialEq)]
pub enum BulletinMenuAction {
    Post,
    ToggleReadFilter,
    ToggleUnreadOnly,
    Read(u32),
    List,
    BackToMenu,
    Submit { title: String, content: String },
    PostContent(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Menu(MenuAction),
    Bulletin(BulletinMenuAction),
}

/// Bulletin menu states
#[derive(Debug, Clone)]
pub enum BulletinMenuState {
    MainMenu,
    Listing(Vec<Bulletin>),
    Reading(Bulletin),      // Reading specific bulletin ID
    Posting,                // Posting new bulletin
    PostingContent(String), // Posting - have title, getting content
}

/// Bulletin menu - can have state like current view, filters, etc.
pub struct BulletinMenu {
    pub state: BulletinMenuState,
    show_read_bulletins: bool,
    show_only_unread: bool,
}

impl BulletinMenu {
    pub fn new() -> Self {
        Self {
            state: BulletinMenuState::MainMenu,
            show_read_bulletins: true,
            show_only_unread: false,
        }
    }

    // // Reset to main bulletin menu
    // pub fn reset(&mut self) {
    //     self.state = BulletinMenuState::MainMenu;
    // }

    /// Set filter options
    pub fn toggle_read_filter(&mut self) {
        self.show_read_bulletins = !self.show_read_bulletins;
    }

    pub fn toggle_unread_only(&mut self) {
        self.show_only_unread = !self.show_only_unread;
    }
}

impl MenuScreen for BulletinMenu {
    // fn name(&self) -> &'static str {
    //     "Bulletins"
    // }

    fn render(&self, data: &BbsSession) -> MenuRender {
        if !data.config.features.bulletins_enabled {
            return self.render_disabled_menu(data);
        }

        match &self.state {
            BulletinMenuState::MainMenu => self.render_main_menu(data),
            BulletinMenuState::Listing(list) => self.render_listing_menu(data, list),
            BulletinMenuState::Reading(bulletin) => self.render_reading_menu(data, bulletin),
            BulletinMenuState::Posting => self.render_posting_menu(data),
            BulletinMenuState::PostingContent(title) => {
                self.render_posting_content_menu(data, title)
            }
        }
    }

    fn handle_input(&self, data: &BbsSession, input: &str) -> MenuAction {
        if !data.config.features.bulletins_enabled {
            return self.handle_disabled_input(input);
        }

        let action = match &self.state {
            BulletinMenuState::MainMenu => self.handle_main_input(data, input),
            BulletinMenuState::Listing(list) => self.handle_listing_input(data, input, list),
            BulletinMenuState::Reading(bulletin) => {
                self.handle_reading_input(data, input, bulletin)
            }
            BulletinMenuState::Posting => self.handle_posting_input(data, input),
            BulletinMenuState::PostingContent(title) => {
                self.handle_posting_content_input(data, input, title)
            }
        };

        // TODO: Not sure this is best way to do this...
        // Translate Action to the appropriate MenuAction to be handled
        // in the session controller
        match action {
            Action::Menu(m) => m,

            Action::Bulletin(b) => match b {
                BulletinMenuAction::Post => MenuAction::BulletinPost,
                BulletinMenuAction::Read(id) => MenuAction::BulletinRead(id),
                BulletinMenuAction::Submit { title, content } => {
                    MenuAction::BulletinSubmit { title, content }
                }
                BulletinMenuAction::PostContent(title) => MenuAction::BulletinPostContent(title),
                BulletinMenuAction::List => MenuAction::BulletinList,
                BulletinMenuAction::BackToMenu => MenuAction::BulletinBackToMenu,
                BulletinMenuAction::ToggleReadFilter => MenuAction::BulletinToggleReadFilter,
                BulletinMenuAction::ToggleUnreadOnly => MenuAction::BulletinToggleUnreadOnly,
            },
        }
    }
}

impl BulletinMenu {
    fn render_disabled_menu(&self, data: &BbsSession) -> MenuRender {
        let items = vec![
            MenuItem::info("!  Bulletin Board has been disabled by the SysOp."),
            MenuItem::info(&format!(
                "Contact {} for more information.",
                data.config.bbs.sysop_name
            )),
            MenuItem::separator(),
            MenuItem::option("B", "Back to main menu"),
        ];
        MenuRender::with_items("FEATURE DISABLED", items, "\nChoice: ")
    }

    fn render_main_menu(&self, data: &BbsSession) -> MenuRender {
        let mut items = vec![];

        // Show bulletin statistics
        let stats = match &data.bulletin_stats {
            Some(s) => s,
            None => &BulletinStats::default(),
        };

        items.push(MenuItem::info(&format!(
            "{} Total | {} Unread",
            stats.total_bulletins, stats.unread_count
        )));

        if stats.total_bulletins == 0 {
            items.push(MenuItem::separator());
            items.push(MenuItem::info("No bulletins posted yet."));
            items.push(MenuItem::info("Be the first to post a bulletin!"));
        } else {
            items.push(MenuItem::separator());
            items.push(MenuItem::info("> Recent Bulletins:"));
            items.push(MenuItem::info(""));

            // Show recent bulletins
            for (i, summary) in stats.recent_bulletins.iter().take(5).enumerate() {
                let status = if summary.is_sticky {
                    "[*]"
                } else if !summary.is_read {
                    "[N]"
                } else {
                    "   "
                };

                let title = if summary.title.len() > 35 {
                    format!("{}...", &summary.title[..32])
                } else {
                    summary.title.clone()
                };

                items.push(MenuItem::info(&format!(
                    "{} [{}] {} - by {} ({})",
                    status,
                    i + 1,
                    title,
                    summary.author,
                    summary.posted_display
                )));
            }
        }

        items.push(MenuItem::separator());

        // Show filter status
        if !self.show_read_bulletins {
            items.push(MenuItem::info("(Hiding read bulletins)"));
        }
        if self.show_only_unread {
            items.push(MenuItem::info("(Showing only unread bulletins)"));
        }

        // Menu options
        if stats.total_bulletins > 0 {
            items.push(MenuItem::option("L", "List all bulletins"));
            items.push(MenuItem::option("#", "Read bulletin by number"));

            if stats.unread_count > 0 {
                items.push(MenuItem::option("N", "Read next unread"));
            }
        }

        // Posting options
        if data.is_logged_in() || data.allow_anonymous() {
            items.push(MenuItem::option("P", "Post new bulletin"));
        } else {
            items.push(MenuItem::disabled_option(
                "P",
                "Post new bulletin (login required)",
            ));
        }

        // Filter options
        if stats.total_bulletins > 0 {
            if self.show_read_bulletins {
                items.push(MenuItem::option("H", "Hide read bulletins"));
            } else {
                items.push(MenuItem::option("S", "Show read bulletins"));
            }

            if !self.show_only_unread {
                items.push(MenuItem::option("U", "Show only unread"));
            } else {
                items.push(MenuItem::option("A", "Show all bulletins"));
            }
        }

        items.push(MenuItem::option("B", "Back to main"));

        MenuRender::with_items("BULLETIN BOARD", items, "\nChoice: ")
    }

    fn render_listing_menu(&self, data: &BbsSession, list: &[Bulletin]) -> MenuRender {
        let mut menu = vec![];

        // Show bulletin statistics
        let stats = match &data.bulletin_stats {
            Some(s) => s,
            None => &BulletinStats::default(),
        };

        menu.push(MenuItem::info(&format!(
            "{} Total | {} Unread",
            stats.total_bulletins, stats.unread_count
        )));

        if stats.total_bulletins == 0 {
            menu.push(MenuItem::separator());
            menu.push(MenuItem::info("No bulletins posted yet."));
            menu.push(MenuItem::info("Be the first to post a bulletin!"));
        } else {
            menu.push(MenuItem::separator());
            menu.push(MenuItem::info("> Recent Bulletins:"));
            menu.push(MenuItem::info(""));

            // Show recent bulletins
            for (i, summary) in list.iter().enumerate() {
                let status = if summary.is_sticky {
                    "[*]"
                } else if !summary.is_read_by(&data.display_username()) {
                    "[N]"
                } else {
                    "   "
                };

                let title = if summary.title.len() > 35 {
                    format!("{}...", &summary.title[..32])
                } else {
                    summary.title.clone()
                };

                menu.push(MenuItem::info(&format!(
                    "{} [{}] {} - by {} ({})",
                    status,
                    i + 1,
                    title,
                    summary.author,
                    summary.posted_display()
                )));
            }
        }

        menu.push(MenuItem::separator());

        // Show filter status
        if !self.show_read_bulletins {
            menu.push(MenuItem::info("(Hiding read bulletins)"));
        }
        if self.show_only_unread {
            menu.push(MenuItem::info("(Showing only unread bulletins)"));
        }

        // Menu options
        if stats.total_bulletins > 0 {
            menu.push(MenuItem::option("#", "Read bulletin by number"));

            if stats.unread_count > 0 {
                menu.push(MenuItem::option("N", "Read next unread"));
            }
        }

        // Posting options
        if data.is_logged_in() || data.allow_anonymous() {
            menu.push(MenuItem::option("P", "Post new bulletin"));
        } else {
            menu.push(MenuItem::disabled_option(
                "P",
                "Post new bulletin (login required)",
            ));
        }

        // Filter options
        if stats.total_bulletins > 0 {
            if self.show_read_bulletins {
                menu.push(MenuItem::option("H", "Hide read bulletins"));
            } else {
                menu.push(MenuItem::option("S", "Show read bulletins"));
            }

            if !self.show_only_unread {
                menu.push(MenuItem::option("U", "Show only unread"));
            } else {
                menu.push(MenuItem::option("A", "Show all bulletins"));
            }
        }

        menu.push(MenuItem::option("B", "Back to main"));

        MenuRender::with_items("ALL BULLETINS", menu, "\nChoice: ")
    }

    fn render_reading_menu(&self, data: &BbsSession, bulletin: &Bulletin) -> MenuRender {
        let mut items = vec![
            MenuItem::info(&format!("Bulletin #{}: {}", bulletin.id, bulletin.title)),
            MenuItem::info(&format!("Author: {}", bulletin.author)),
            MenuItem::info(&format!("Posted: {}", bulletin.posted_display())),
            MenuItem::separator(),
        ];

        // Show the full content - we'll need to load it from storage
        items.push(MenuItem::info("Content:"));
        items.push(MenuItem::info(""));

        // TODO: In a real implementation, we'd fetch the full bulletin content here
        // For now, we'll show placeholder content based on the summary
        // let content_lines = vec![
        //     "This is the bulletin content that would be loaded from storage.",
        //     "Each line of the bulletin would be displayed here with proper",
        //     "formatting and word wrapping as needed for the terminal width.",
        //     "",
        //     "The bulletin system supports rich content including:",
        //     "- Multiple paragraphs",
        //     "- Lists and formatting",
        //     "- Special characters and symbols",
        //     "",
        //     "[This is placeholder content - real implementation would load",
        //     "the actual bulletin text from the storage system.]",
        // ];

        let width = &data.config.ui.menu_width;
        for line in bulletin.get_content_lines(*width-4) {
            items.push(MenuItem::info(&line));
        }

        items.push(MenuItem::separator());
        items.push(MenuItem::option("N", "Next bulletin"));
        items.push(MenuItem::option("P", "Previous bulletin"));
        items.push(MenuItem::option("L", "List all bulletins"));
        items.push(MenuItem::option("B", "Back to bulletin menu"));

        MenuRender::with_items(
            &format!("READING BULLETIN #{}", bulletin.id),
            items,
            "\nChoice: ",
        )
    }

    fn render_posting_menu(&self, data: &BbsSession) -> MenuRender {
        let author = data.display_username();

        let items = vec![
            MenuItem::info(&format!("Posting as: {}", author)),
            MenuItem::info(""),
            MenuItem::info("Enter bulletin title (max 100 characters):"),
            MenuItem::info("- Keep it descriptive but concise"),
            MenuItem::info("- Avoid ALL CAPS unless necessary"),
            MenuItem::info(""),
            MenuItem::separator(),
            MenuItem::info("Press Enter with empty title to cancel"),
        ];

        MenuRender::with_items("POST NEW BULLETIN", items, "\nTitle: ")
    }

    fn render_posting_content_menu(&self, data: &BbsSession, title: &str) -> MenuRender {
        let author = data.display_username();

        let items = vec![
            MenuItem::info(&format!("Posting as: {}", author)),
            MenuItem::info(&format!("Title: {}", title)),
            MenuItem::info(""),
            MenuItem::info(&format!(
                "Enter bulletin content (max {} characters):",
                data.config.features.max_message_length
            )),
            MenuItem::info("- Write your message clearly"),
            MenuItem::info("- Be respectful of other users"),
            MenuItem::info("- No spam or inappropriate content"),
            MenuItem::info(""),
            MenuItem::separator(),
            MenuItem::info("Press Enter with empty content to cancel"),
        ];

        MenuRender::with_items("POST BULLETIN - CONTENT", items, "\nContent: ")
    }

    fn handle_disabled_input(&self, input: &str) -> MenuAction {
        match input.to_lowercase().as_str() {
            "b" => MenuAction::GoTo(Menu::Main),
            _ => MenuAction::ShowMessage(
                "Bulletin Board is disabled. Press B to go back.".to_string(),
            ),
        }
    }

    fn handle_main_input(&self, data: &BbsSession, input: &str) -> Action {
        match input.to_lowercase().as_str() {
            // "l" => Action::Menu(MenuAction::ShowMessage(
            //     "Listing all bulletins... (Feature integration needed!)".to_string(),
            // )),
            "l" => {
                if data.is_logged_in() || data.allow_anonymous() {
                    Action::Bulletin(BulletinMenuAction::List)
                } else {
                    Action::Menu(MenuAction::ShowMessage(
                        "You must be logged in to list bulletins.".to_string(),
                    ))
                }
            }
            "r" => Action::Menu(MenuAction::ShowMessage(
                "Enter bulletin number to read... (Feature integration needed!)".to_string(),
            )),
            "n" => Action::Menu(MenuAction::ShowMessage(
                "Reading next unread bulletin... (Feature integration needed!)".to_string(),
            )),
            "p" => {
                if data.is_logged_in() || data.allow_anonymous() {
                    Action::Bulletin(BulletinMenuAction::Post)
                } else {
                    Action::Menu(MenuAction::ShowMessage(
                        "You must be logged in to post bulletins.".to_string(),
                    ))
                }
            }
            "h" => Action::Bulletin(BulletinMenuAction::ToggleReadFilter),
            "s" => Action::Bulletin(BulletinMenuAction::ToggleReadFilter),
            "u" => Action::Bulletin(BulletinMenuAction::ToggleUnreadOnly),
            "a" => Action::Bulletin(BulletinMenuAction::ToggleUnreadOnly),
            "b" => Action::Menu(MenuAction::GoTo(Menu::Main)),
            // Handle reading specific bulletin numbers
            num if num.chars().all(|c| c.is_ascii_digit()) => {
                if let Ok(bulletin_id) = num.parse::<u32>() {
                    Action::Bulletin(BulletinMenuAction::Read(bulletin_id))
                } else {
                    Action::Menu(MenuAction::ShowMessage(
                        "Invalid bulletin number.".to_string(),
                    ))
                }
            }
            _ => Action::Menu(MenuAction::ShowMessage(
                "Invalid choice. Use L, R, N, P, H/S, U/A, or B.".to_string(),
            )),
        }
    }

    fn handle_listing_input(&self, data: &BbsSession, input: &str, _list: &[Bulletin]) -> Action {
        match input.to_lowercase().as_str() {
            "n" => Action::Menu(MenuAction::ShowMessage(
                "Reading next unread bulletin... (Feature integration needed!)".to_string(),
            )),
            "p" => {
                if data.is_logged_in() || data.allow_anonymous() {
                    Action::Bulletin(BulletinMenuAction::Post)
                } else {
                    Action::Menu(MenuAction::ShowMessage(
                        "You must be logged in to post bulletins.".to_string(),
                    ))
                }
            }
            "h" => Action::Bulletin(BulletinMenuAction::ToggleReadFilter),
            "s" => Action::Bulletin(BulletinMenuAction::ToggleReadFilter),
            "u" => Action::Bulletin(BulletinMenuAction::ToggleUnreadOnly),
            "a" => Action::Bulletin(BulletinMenuAction::ToggleUnreadOnly),
            "b" => Action::Bulletin(BulletinMenuAction::BackToMenu),
            // Handle reading specific bulletin numbers
            num if num.chars().all(|c| c.is_ascii_digit()) => {
                if let Ok(bulletin_id) = num.parse::<u32>() {
                    Action::Bulletin(BulletinMenuAction::Read(bulletin_id))
                } else {
                    Action::Menu(MenuAction::ShowMessage(
                        "Invalid bulletin number.".to_string(),
                    ))
                }
            }
            _ => Action::Menu(MenuAction::ShowMessage(
                "Invalid choice. Use L, R, N, P, H/S, U/A, or B.".to_string(),
            )),
        }
    }

    fn handle_reading_input(
        &self,
        _data: &BbsSession,
        input: &str,
        _bulletin: &Bulletin,
    ) -> Action {
        match input.to_lowercase().as_str() {
            "n" => Action::Menu(MenuAction::ShowMessage(
                "Next bulletin... (Feature integration needed!)".to_string(),
            )),
            "p" => Action::Menu(MenuAction::ShowMessage(
                "Previous bulletin... (Feature integration needed!)".to_string(),
            )),
            "l" => Action::Bulletin(BulletinMenuAction::List),
            "b" => Action::Bulletin(BulletinMenuAction::BackToMenu),
            _ => Action::Menu(MenuAction::ShowMessage(
                "Invalid choice. Use N, P, L, or B.".to_string(),
            )),
        }
    }

    fn handle_posting_input(&self, _data: &BbsSession, input: &str) -> Action {
        if input.trim().is_empty() {
            Action::Bulletin(BulletinMenuAction::BackToMenu)
        } else if input.len() > 100 {
            Action::Menu(MenuAction::ShowMessage(
                "Title too long (max 100 characters). Try again.".to_string(),
            ))
        } else {
            Action::Bulletin(BulletinMenuAction::PostContent(input.trim().to_string()))
        }
    }

    fn handle_posting_content_input(&self, data: &BbsSession, input: &str, title: &str) -> Action {
        if input.trim().is_empty() {
            Action::Bulletin(BulletinMenuAction::BackToMenu)
        } else if input.len() > data.config.features.max_message_length {
            Action::Menu(MenuAction::ShowMessage(format!(
                "Content too long (max {} characters). Try again.",
                data.config.features.max_message_length
            )))
        } else {
            Action::Bulletin(BulletinMenuAction::Submit {
                title: title.to_string(),
                content: input.trim().to_string(),
            })
        }
    }
}
