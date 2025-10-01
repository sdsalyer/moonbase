use crate::box_renderer::{BoxRenderer, BoxStyle};
use crate::bulletin_repository::{JsonBulletinStorage, BulletinStorage, BulletinStats};
use crate::config::BbsConfig;
use crate::errors::{BbsError, BbsResult};
use crate::menu::{Menu, MenuAction, MenuRender, MenuScreen, RecentLogin, UserStats};
use crate::user_repository::{JsonUserStorage};
use crate::users::{RegistrationRequest, User};

use crossterm::{
    QueueableCommand, cursor,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct BbsSession {
    pub config: Arc<BbsConfig>,
    pub user: Option<User>,
    pub menu_current: Menu,
    pub user_stats: Option<UserStats>,
    pub bulletin_stats: Option<BulletinStats>,

    // Session resources
    user_storage: Arc<Mutex<JsonUserStorage>>,
    bulletin_storage: Arc<Mutex<JsonBulletinStorage>>,
    box_renderer: BoxRenderer,
    login_attempts: u8,

    // Menu instances (owned by session, can maintain state)
    menu_main: crate::menu::menu_main::MainMenu,
    menu_bulletin: crate::menu::menu_bulletin::BulletinMenu,
    menu_user: crate::menu::menu_user::UserMenu,
    // menu_message: crate::menu::menu_message::MessageMenu,
    // menu_file: crate::menu::menu_file::FileMenu,
}

impl BbsSession {
    pub fn new(config: Arc<BbsConfig>, user_storage: Arc<Mutex<JsonUserStorage>>, bulletin_storage: Arc<Mutex<JsonBulletinStorage>>) -> Self {
        let box_renderer = BoxRenderer::new(BoxStyle::Ascii, config.ui.use_colors);

        Self {
            config,
            user: None,
            menu_current: Menu::Main,
            user_storage,
            bulletin_storage,
            user_stats: None,
            bulletin_stats: None,
            box_renderer,
            login_attempts: 0,
            menu_main: crate::menu::menu_main::MainMenu::new(),
            menu_bulletin: crate::menu::menu_bulletin::BulletinMenu::new(),
            menu_user: crate::menu::menu_user::UserMenu::new(),
            // menu_message: crate::menu::menu_message::MessageMenu::new(),
            // menu_file: crate::menu::menu_file::FileMenu::new(),
        }
    }

    pub fn is_logged_in(&self) -> bool {
        self.user.is_some()
    }

    /// Helper to check if anonymous access is allowed
    pub fn allow_anonymous(&self) -> bool {
        self.config.features.allow_anonymous
    }

    /// Get the current username, or "Anonymous" if not logged in
    pub fn display_username(&self) -> String {
        match &self.user {
            // TODO: why clone
            Some(u) => u.username.clone(),
            None => "Anonymous".to_string(),
        }
    }

    /// Run the BBS session with the provided stream
    pub fn run(&mut self, mut stream: TcpStream) -> BbsResult<()> {
        // Set initial timeout
        stream.set_read_timeout(Some(self.config.timeouts.connection_timeout))?;

        // Initialize terminal
        self.initialize_terminal(&mut stream)?;

        // Show welcome screen
        self.show_welcome(&mut stream)?;

        // Check if anonymous access is allowed
        if !self.config.features.allow_anonymous && self.user.is_none() {
            self.force_login(&mut stream)?;
        }

        // Initialize stats
        let _ = self.refresh_bulletin_stats();

        // Main session loop
        loop {
            if !self.menu_handle_loop(&mut stream)? {
                break; // User chose to quit
            }
        }

        Ok(())
    }

    /// Get the current menu instance
    fn menu_get_current(&self) -> &dyn MenuScreen {
        match self.menu_current {
            Menu::Main => &self.menu_main,
            Menu::Bulletins => &self.menu_bulletin,
            Menu::Users => &self.menu_user,
            // CurrentMenu::Messages => &self.menu_message,
            // CurrentMenu::Files => &self.menu_file,
        }
    }

    /// Calculate current user statistics
    fn calculate_user_stats(&mut self) -> BbsResult<()> {
        let storage = self
            .user_storage
            .lock()
            .map_err(|_| BbsError::Configuration("Storage lock poisoned".to_string()))?;

        let stats = storage.get_stats()?;
        let total_users = stats.total_users;
        let all_users = stats.all_users;
        let online_users = stats.online_users;

        // Get recent logins (limit to 5 most recent)
        let mut recent_logins = stats.recent_logins;

        // Add current user if logged in
        if let Some(ref current_user) = self.user {
            recent_logins.push(RecentLogin {
                username: current_user.username.clone(),
                last_login_display: "just now".to_string(),
                is_current_user: true,
            });
        } else {
            // Add anonymous user
            recent_logins.push(RecentLogin {
                username: "Anonymous".to_string(),
                last_login_display: "just now".to_string(),
                is_current_user: true,
            });
        }

        // TODO: query the storage for actual recent logins sorted by last_login timestamp
        // For now, add the sysop as an example
        recent_logins.push(RecentLogin {
            username: "SysOp".to_string(),
            last_login_display: "2 hours ago".to_string(),
            is_current_user: false,
        });

        self.user_stats = Some(UserStats {
            total_users,
            online_users,
            all_users,
            recent_logins,
        });

        Ok(())
    }

    /// Main menu loop - render, display, get input, handle action
    fn menu_handle_loop(&mut self, stream: &mut TcpStream) -> BbsResult<bool> {

        // 1. Check user stats
        // This has to come first because of the mutable borrow
        let _ = self.calculate_user_stats();

        // 2. Get current menu
        let menu_current = self.menu_get_current();

        // 3. Render menu (pure function, returns data)
        let menu_render = menu_current.render(&self);

        // 4. Display menu (session handles I/O)
        let _ = self.menu_show(stream, &menu_render)?;

        // 5. Get input (session handles I/O)
        let input = self.get_input(stream, &menu_render.prompt)?;

        // 6. Handle input (pure function, returns action)
        let action = menu_current.handle_input(&self, &input);

        // 7. Process action (session handles state changes)
        self.menu_handle_action(stream, action)
    }

    /// Process menu actions and update session state
    fn menu_handle_action(
        &mut self,
        stream: &mut TcpStream,
        action: MenuAction,
    ) -> BbsResult<bool> {
        match action {
            // MenuAction::Stay => Ok(true),
            MenuAction::GoTo(menu) => {
                self.menu_current = menu;
                Ok(true)
            }
            MenuAction::Login => {
                self.handle_login(stream)?;
                Ok(true)
            }
            MenuAction::Logout => {
                self.user = None;
                self.show_message_with_stream(
                    stream,
                    "SYSTEM MESSAGE",
                    "You have been logged out.",
                    Some(Color::Yellow),
                )?;
                Ok(true)
            }
            MenuAction::Quit => {
                self.show_goodbye(stream)?;
                Ok(false)
            }
            MenuAction::ShowMessage(message) => {
                self.show_message_with_stream(
                    stream,
                    "SYSTEM MESSAGE",
                    &message,
                    Some(Color::Yellow),
                )?;
                Ok(true)
            }
            // Bulletin-specific actions
            MenuAction::BulletinPost => {
                self.menu_bulletin.state = crate::menu::menu_bulletin::BulletinMenuState::Posting;
                Ok(true)
            }
            MenuAction::BulletinRead(id) => {
                self.handle_bulletin_read(stream, id)?;
                Ok(true)
            }
            MenuAction::BulletinSubmit { title, content } => {
                self.handle_bulletin_submit(stream, title, content)?;
                Ok(true)
            }
            MenuAction::BulletinPostContent(title) => {
                self.menu_bulletin.state = crate::menu::menu_bulletin::BulletinMenuState::PostingContent(title);
                Ok(true)
            }
            MenuAction::BulletinList => {
                self.menu_bulletin.state = crate::menu::menu_bulletin::BulletinMenuState::MainMenu;
                self.refresh_bulletin_stats()?;
                Ok(true)
            }
            MenuAction::BulletinBackToMenu => {
                self.menu_bulletin.state = crate::menu::menu_bulletin::BulletinMenuState::MainMenu;
                Ok(true)
            }
            MenuAction::BulletinToggleReadFilter => {
                self.menu_bulletin.toggle_read_filter();
                self.refresh_bulletin_stats()?;
                Ok(true)
            }
            MenuAction::BulletinToggleUnreadOnly => {
                self.menu_bulletin.toggle_unread_only();
                self.refresh_bulletin_stats()?;
                Ok(true)
            }
        }
    }

    /// Get user input with a prompt
    fn get_input(&self, stream: &mut TcpStream, prompt: &str) -> BbsResult<String> {
        stream.queue(Print(prompt))?;
        stream.flush()?;

        let mut buffer = [0; 1024];
        match stream.read(&mut buffer) {
            Ok(0) => Err(BbsError::ClientDisconnected),
            Ok(n) => {
                let input = String::from_utf8_lossy(&buffer[0..n]);
                Ok(input.trim().to_string())
            }
            Err(e) => Err(BbsError::from(e)),
        }
    }

    /// Initialize terminal state
    fn initialize_terminal(&mut self, stream: &mut TcpStream) -> BbsResult<()> {
        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;
        stream.flush()?;
        Ok(())
    }

    /// Show the welcome screen
    fn show_welcome(&mut self, stream: &mut TcpStream) -> BbsResult<()> {
        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;

        let welcome_msg = format!(
            r#"
*  {}  *

{}
SysOp: {} | Est. {}
Location: {}
"#,
            self.config.bbs.name.chars().take(30).collect::<String>(),
            self.config.bbs.tagline.chars().take(50).collect::<String>(),
            self.config.bbs.sysop_name,
            self.config.bbs.established,
            self.config.bbs.location
        );

        self.box_renderer.render_message_box(
            stream,
            "WELCOME",
            &welcome_msg,
            self.config.ui.menu_width,
            Some(Color::Magenta),
        )?;

        stream.queue(Print("\nPress Enter to continue..."))?;
        stream.flush()?;

        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer);

        Ok(())
    }

    /// Handle user login process
    fn handle_login(&mut self, stream: &mut TcpStream) -> BbsResult<()> {
        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;

        let instructions = "Choose an option:\n\n[L] Login with existing account\n[R] Register new account\n[C] Cancel";
        self.box_renderer.render_message_box(
            stream,
            "LOGIN / REGISTER",
            instructions,
            self.config.ui.menu_width,
            Some(Color::Cyan),
        )?;

        let choice = self.get_input(stream, "\nChoice: ")?;

        match choice.to_lowercase().as_str() {
            "l" | "login" => self.handle_existing_login(stream),
            "r" | "register" => self.handle_registration(stream),
            "c" | "cancel" => {
                self.show_message_with_stream(
                    stream,
                    "LOGIN",
                    "Login cancelled.",
                    Some(Color::Yellow),
                )?;
                Ok(())
            }
            _ => {
                self.show_message_with_stream(
                    stream,
                    "ERROR",
                    "Invalid choice. Please try again.",
                    Some(Color::Red),
                )?;
                Ok(())
            }
        }
    }

    /// Handle login for existing user
    fn handle_existing_login(&mut self, stream: &mut TcpStream) -> BbsResult<()> {
        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;

        self.box_renderer.render_message_box(
            stream,
            "USER LOGIN",
            "Enter your credentials:",
            self.config.ui.menu_width,
            Some(Color::Cyan),
        )?;

        let username = self.get_input(stream, "\nUsername: ")?;
        if username.is_empty() {
            self.show_message_with_stream(
                stream,
                "LOGIN",
                "Login cancelled.",
                Some(Color::Yellow),
            )?;
            return Ok(());
        }

        let password = self.get_input(stream, "Password: ")?;
        if password.is_empty() {
            self.show_message_with_stream(
                stream,
                "LOGIN",
                "Login cancelled.",
                Some(Color::Yellow),
            )?;
            return Ok(());
        }

        // Authenticate user
        let registration_result = {
            // this scope allows the mutex guard to be dropped
            // so we can reference self later
            let mut storage = self
                .user_storage
                .lock()
                .map_err(|_| BbsError::Configuration("Storage lock poisoned".to_string()))?;
            storage.authenticate_user(&username, &password)
        };

        match registration_result? {
            Some(user) => {
                self.user = Some(user.clone());
                let welcome_msg = format!(
                    "Welcome back, {}!\n\nLast login: {}\nTotal logins: {}",
                    user.username,
                    user.last_login_display(),
                    user.login_count
                );
                self.show_message_with_stream(
                    stream,
                    "LOGIN SUCCESS",
                    &welcome_msg,
                    Some(Color::Green),
                )
            }
            None => self.show_message_with_stream(
                stream,
                "LOGIN FAILED",
                "Invalid username or password.",
                Some(Color::Red),
            ),
        }
    }

    /// Handle new user registration
    fn handle_registration(&mut self, stream: &mut TcpStream) -> BbsResult<()> {
        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;

        let instructions = format!(
            "Create your account:\n\nUsername rules:\n- 1-{} characters\n- Letters, numbers, underscore only\n- Must be unique",
            self.config.features.max_username_length
        );
        self.box_renderer.render_message_box(
            stream,
            "USER REGISTRATION",
            &instructions,
            self.config.ui.menu_width,
            Some(Color::Cyan),
        )?;

        // Get username
        let username = self.get_input(stream, "\nUsername: ")?;
        if username.is_empty() {
            self.show_message_with_stream(
                stream,
                "REGISTRATION",
                "Registration cancelled.",
                Some(Color::Yellow),
            )?;
            return Ok(());
        }

        // Get password
        let password = self.get_input(stream, "Password (min 4 chars): ")?;
        if password.is_empty() {
            self.show_message_with_stream(
                stream,
                "REGISTRATION",
                "Registration cancelled.",
                Some(Color::Yellow),
            )?;
            return Ok(());
        }

        // Get optional email
        let email_input = self.get_input(stream, "Email (optional): ")?;
        let email = if email_input.is_empty() {
            None
        } else {
            Some(email_input)
        };

        // Create registration request
        let request = RegistrationRequest::new(username.clone(), email, password);

        // Attempt registration
        let registration_result = {
            // this scope allows the mutex guard to be dropped
            // so we can reference self later
            let mut storage = self
                .user_storage
                .lock()
                .map_err(|_| BbsError::Configuration("Storage lock poisoned".to_string()))?;
            storage.register_user(&request, &self.config)
        };

        match registration_result {
            Ok(user) => {
                self.user = Some(user.clone());
                let success_msg = format!(
                    "Registration successful!\n\nWelcome to {}, {}!\nYour account has been created and you are now logged in.",
                    self.config.bbs.name, user.username
                );
                self.show_message_with_stream(
                    stream,
                    "REGISTRATION SUCCESS",
                    &success_msg,
                    Some(Color::Green),
                )
            }
            Err(e) => {
                let error_msg = format!("Registration failed: {}", e);
                self.show_message_with_stream(
                    stream,
                    "REGISTRATION FAILED",
                    &error_msg,
                    Some(Color::Red),
                )
            }
        }
    }

    /// Force login for restricted BBS
    fn force_login(&mut self, stream: &mut TcpStream) -> BbsResult<()> {
        let message = "This BBS requires registration to access. Anonymous access has been disabled by the SysOp.";

        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;

        self.box_renderer.render_message_box(
            stream,
            "LOGIN REQUIRED",
            message,
            self.config.ui.menu_width,
            Some(Color::Yellow),
        )?;

        let timeout = self.config.timeouts.login_timeout;
        stream.set_read_timeout(Some(timeout))?;

        while self.user.is_none() && self.login_attempts < 3 {
            if !self.attempt_login(stream)? {
                break;
            }
        }

        if self.user.is_none() {
            return Err(BbsError::AuthenticationFailed(
                "Too many failed login attempts".to_string(),
            ));
        }

        stream.set_read_timeout(Some(self.config.timeouts.connection_timeout))?;
        Ok(())
    }

    /// Single login attempt
    fn attempt_login(&mut self, stream: &mut TcpStream) -> BbsResult<bool> {
        self.login_attempts += 1;

        let username = self.get_input(
            stream,
            &format!("Login attempt {}/3\nUsername: ", self.login_attempts),
        )?;

        if username.len() > self.config.features.max_username_length {
            stream.queue(SetForegroundColor(Color::Red))?;
            stream.queue(Print(&format!(
                "Username too long (max {} characters)\n\n",
                self.config.features.max_username_length
            )))?;
            stream.queue(ResetColor)?;
            stream.flush()?;
            return Ok(true);
        }

        if !username.is_empty() {
            let password = self.get_input(stream, "Password: ")?;

            // Try to authenticate
            let mut storage = self
                .user_storage
                .lock()
                .map_err(|_| BbsError::Configuration("Storage lock poisoned".to_string()))?;

            match storage.authenticate_user(&username, &password)? {
                Some(user) => {
                    self.user = Some(user.clone());
                    stream.queue(SetForegroundColor(Color::Green))?;
                    stream.queue(Print(&format!("Welcome, {}!\n\n", user.username)))?;
                    stream.queue(ResetColor)?;
                    stream.flush()?;
                    std::thread::sleep(Duration::from_secs(1));
                    return Ok(false);
                }
                None => {
                    stream.queue(SetForegroundColor(Color::Red))?;
                    stream.queue(Print("Invalid username or password.\n\n"))?;
                    stream.queue(ResetColor)?;
                    stream.flush()?;
                    return Ok(true);
                }
            }
        }

        Ok(true)
    }

    /// Display a rendered menu
    fn menu_show(&self, stream: &mut TcpStream, render: &MenuRender) -> BbsResult<()> {
        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;
        self.box_renderer.render_menu(
            stream,
            &render.title,
            &render.items,
            self.config.ui.menu_width,
            None,
        )?;
        Ok(())
    }

    /// Display a message box with stream
    fn show_message_with_stream(
        &mut self,
        stream: &mut TcpStream,
        title: &str,
        message: &str,
        color: Option<Color>,
    ) -> BbsResult<()> {
        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;

        self.box_renderer.render_message_box(
            stream,
            title,
            message,
            self.config.ui.menu_width,
            color,
        )?;

        stream.queue(Print("\nPress Enter to continue..."))?;
        stream.flush()?;

        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer);

        Ok(())
    }

    /// Show goodbye screen
    fn show_goodbye(&mut self, stream: &mut TcpStream) -> BbsResult<()> {
        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;

        let goodbye_msg = format!(
            "Thanks for visiting {}!\n\nSysOp: {}\n\n* Come back anytime! *",
            self.config.bbs.name, self.config.bbs.sysop_name
        );

        self.box_renderer.render_message_box(
            stream,
            "GOODBYE",
            &goodbye_msg,
            self.config.ui.menu_width,
            Some(Color::Magenta),
        )?;

        stream.queue(Print("\nConnection will close in 3 seconds...\n"))?;
        stream.flush()?;

        std::thread::sleep(Duration::from_secs(3));
        Ok(())
    }

    /// Handle bulletin reading
    fn handle_bulletin_read(&mut self, stream: &mut TcpStream, id: u32) -> BbsResult<()> {
        // Load bulletin from storage
        let bulletin = {
            let storage = self
                .bulletin_storage
                .lock()
                .map_err(|_| BbsError::Configuration("Storage lock poisoned".to_string()))?;
            storage.load_bulletin(id)?
        };

        match bulletin {
            Some(_bulletin) => {
                // Mark as read for logged-in users
                if let Some(user) = &self.user {
                    let mut storage = self
                        .bulletin_storage
                        .lock()
                        .map_err(|_| BbsError::Configuration("Storage lock poisoned".to_string()))?;
                    storage.mark_read(id, &user.username)?;
                }

                // Set menu to reading state
                self.menu_bulletin.state = crate::menu::menu_bulletin::BulletinMenuState::Reading(id);
                
                // Store bulletin content for display
                // Note: In a real implementation, you might want to store this in the session
                // For now, the menu will fetch it again when rendering
                
                self.refresh_bulletin_stats()?;
                Ok(())
            }
            None => {
                self.show_message_with_stream(
                    stream,
                    "BULLETIN NOT FOUND",
                    &format!("Bulletin #{} was not found.", id),
                    Some(Color::Red),
                )?;
                Ok(())
            }
        }
    }

    /// Handle bulletin submission
    fn handle_bulletin_submit(
        &mut self,
        stream: &mut TcpStream,
        title: String,
        content: String,
    ) -> BbsResult<()> {
        let author = self.display_username();
        
        // Create bulletin request
        let request = crate::bulletins::BulletinRequest::new(title.clone(), content, author);

        // Post bulletin
        let result = {
            let mut storage = self
                .bulletin_storage
                .lock()
                .map_err(|_| BbsError::Configuration("Storage lock poisoned".to_string()))?;
            storage.post_bulletin(&request, &self.config)
        };

        match result {
            Ok(bulletin_id) => {
                self.show_message_with_stream(
                    stream,
                    "BULLETIN POSTED",
                    &format!("Your bulletin '{}' has been posted as #{}", title, bulletin_id),
                    Some(Color::Green),
                )?;
                
                // Reset menu state and refresh stats
                self.menu_bulletin.state = crate::menu::menu_bulletin::BulletinMenuState::MainMenu;
                self.refresh_bulletin_stats()?;
                Ok(())
            }
            Err(e) => {
                self.show_message_with_stream(
                    stream,
                    "POSTING FAILED",
                    &format!("Failed to post bulletin: {}", e),
                    Some(Color::Red),
                )?;
                Ok(())
            }
        }
    }

    /// Refresh bulletin statistics
    fn refresh_bulletin_stats(&mut self) -> BbsResult<()> {
        let current_user = self.user.as_ref().map(|u| u.username.as_str());
        
        let stats = {
            let storage = self
                .bulletin_storage
                .lock()
                .map_err(|_| BbsError::Configuration("Storage lock poisoned".to_string()))?;
            storage.get_stats(current_user)
        };

        self.bulletin_stats = Some(stats);
        Ok(())
    }

    // Show feature disabled message
    // fn show_feature_disabled(
    //     &mut self,
    //     stream: &mut TcpStream,
    //     feature_name: &str,
    // ) -> BbsResult<()> {
    //     let width = self.config.ui.menu_width + 20;
    //     let message = format!(
    //         "!  {} has been disabled by the SysOp.\n\nContact {} for more information.",
    //         feature_name, self.config.bbs.sysop_name
    //     );
    //
    //     stream.queue(Clear(ClearType::All))?;
    //     stream.queue(cursor::MoveTo(0, 0))?;
    //     self.box_renderer.render_message_box(
    //         stream,
    //         "FEATURE DISABLED",
    //         &message,
    //         width,
    //         Some(Color::Red),
    //     )?;
    //
    //     stream.queue(Print("\nPress Enter to continue..."))?;
    //     stream.flush()?;
    //
    //     let mut buffer = [0; 1024];
    //     let _ = stream.read(&mut buffer);
    //
    //     Ok(())
    // }
}
