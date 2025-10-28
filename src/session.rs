use crate::box_renderer::{BoxRenderer, BoxStyle};
use crate::bulletin_repository::BulletinStats;
use crate::config::BbsConfig;
use crate::errors::{BbsError, BbsResult};
use crate::menu::{Menu, MenuAction, MenuRender, MenuScreen, RecentLogin, UserStats};

use crate::bulletins::Bulletin;
use crate::users::{RegistrationRequest, User};

use crossterm::{
    QueueableCommand, cursor,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Duration;

// Phase 5: Use TelnetStream for transparent telnet handling
// Phase 7: Import terminal capabilities for adaptive UI
use telnet_negotiation::{TelnetStream, TerminalCapabilities};

pub struct BbsSession {
    pub config: Arc<BbsConfig>,
    pub user: Option<User>,
    pub menu_current: Menu,
    pub user_stats: Option<UserStats>,
    pub bulletin_stats: Option<BulletinStats>,

    // Session resources
    pub services: Arc<crate::services::CoreServices>,
    box_renderer: BoxRenderer,
    login_attempts: u8,

    // Phase 7: Terminal capabilities for adaptive UI
    terminal_capabilities: TerminalCapabilities,
    effective_width: usize,

    // Menu instances (owned by session, can maintain state)
    menu_main: crate::menu::menu_main::MainMenu,
    menu_bulletin: crate::menu::menu_bulletin::BulletinMenu,
    menu_user: crate::menu::menu_user::UserMenu,
    menu_message: crate::menu::menu_message::MessageMenu,
    // menu_file: crate::menu::menu_file::FileMenu,
}

impl BbsSession {
    pub fn new(config: Arc<BbsConfig>, services: Arc<crate::services::CoreServices>) -> Self {
        let box_renderer = BoxRenderer::new(BoxStyle::Ascii, config.ui.use_colors);

        Self {
            config: config.clone(),
            user: None,
            menu_current: Menu::Main,
            user_stats: None,
            bulletin_stats: None,

            // Session resources
            services,
            box_renderer,
            login_attempts: 0,

            // Phase 7: Initialize terminal capabilities
            terminal_capabilities: TerminalCapabilities::default(),
            effective_width: config.ui.width_value,

            menu_main: crate::menu::menu_main::MainMenu::new(),
            menu_bulletin: crate::menu::menu_bulletin::BulletinMenu::new(),
            menu_user: crate::menu::menu_user::UserMenu::new(),
            menu_message: crate::menu::menu_message::MessageMenu::new(),
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

    /// Get the effective terminal width for rendering
    pub fn effective_width(&self) -> usize {
        self.effective_width
    }

    /// Run the BBS session with the provided stream
    pub fn run(&mut self, mut stream: TelnetStream) -> BbsResult<()> {
        // Set initial timeout
        stream.set_read_timeout(Some(self.config.timeouts.connection_timeout))?;

        // Phase 7: Negotiate terminal capabilities
        self.negotiate_terminal_capabilities(&mut stream)?;

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

    /// Phase 7: Negotiate terminal capabilities for adaptive UI
    fn negotiate_terminal_capabilities(&mut self, stream: &mut TelnetStream) -> BbsResult<()> {
        // Get capabilities from TelnetStream
        self.terminal_capabilities = stream.get_terminal_capabilities();

        // Request terminal type if configured for auto-detection
        if matches!(
            self.config.ui.ansi_support,
            crate::config::AutoDetectOption::Auto
        ) || matches!(
            self.config.ui.color_support,
            crate::config::AutoDetectOption::Auto
        ) {
            let _ = stream.request_terminal_type()?;
        }

        // Request window size if configured for auto-detection
        if matches!(self.config.ui.width_mode, crate::config::WidthMode::Auto) {
            let _ = stream.request_window_size()?;
        }

        // Give a moment for negotiation to complete
        std::thread::sleep(Duration::from_millis(100));

        // Update capabilities after negotiation attempts
        self.terminal_capabilities = stream.get_terminal_capabilities();

        // Calculate effective width
        self.effective_width = self.calculate_effective_width();

        // Update box renderer with detected capabilities
        let supports_color = self.resolve_color_support();
        let box_style = self.resolve_box_style();
        self.box_renderer = BoxRenderer::new(box_style, supports_color);

        Ok(())
    }

    /// Calculate the effective terminal width based on configuration and detection
    fn calculate_effective_width(&self) -> usize {
        match &self.config.ui.width_mode {
            crate::config::WidthMode::Auto => {
                if let Some(detected_width) = self.terminal_capabilities.width {
                    detected_width as usize
                } else {
                    self.config.ui.width_value
                }
            }
            crate::config::WidthMode::Fixed => self.config.ui.width_value,
        }
    }

    /// Resolve color support based on configuration and terminal detection
    fn resolve_color_support(&self) -> bool {
        match &self.config.ui.color_support {
            crate::config::AutoDetectOption::Auto => self.terminal_capabilities.supports_color,
            crate::config::AutoDetectOption::Enabled => true,
            crate::config::AutoDetectOption::Disabled => false,
        }
    }

    /// Resolve ANSI support and appropriate box style
    fn resolve_box_style(&self) -> BoxStyle {
        let ansi_supported = match &self.config.ui.ansi_support {
            crate::config::AutoDetectOption::Auto => self.terminal_capabilities.supports_ansi,
            crate::config::AutoDetectOption::Enabled => true,
            crate::config::AutoDetectOption::Disabled => false,
        };

        if ansi_supported {
            // Use configured style if ANSI is supported
            self.config.ui.box_style
        } else {
            // Fall back to ASCII for maximum compatibility
            BoxStyle::Ascii
        }
    }

    /// Detect ANSI support from terminal type string (helper for negotiation)
    fn detect_ansi_support(terminal_type: &str) -> bool {
        let terminal_lower = terminal_type.to_lowercase();
        terminal_lower.contains("xterm")
            || terminal_lower.contains("ansi")
            || terminal_lower.contains("vt100")
            || terminal_lower.contains("linux")
            || terminal_lower.contains("screen")
            || terminal_lower.contains("tmux")
    }

    /// Detect color support from terminal type string (helper for negotiation)
    fn detect_color_support(terminal_type: &str) -> bool {
        let terminal_lower = terminal_type.to_lowercase();
        terminal_lower.contains("xterm")
            || terminal_lower.contains("color")
            || terminal_lower.contains("256")
            || terminal_lower.contains("screen")
            || terminal_lower.contains("tmux")
    }

    /// Get the current menu instance
    fn menu_get_current(&self) -> &dyn MenuScreen {
        match self.menu_current {
            Menu::Main => &self.menu_main,
            Menu::Bulletins => &self.menu_bulletin,
            Menu::Users => &self.menu_user,
            Menu::Messages => &self.menu_message,
            // CurrentMenu::Files => &self.menu_file,
        }
    }

    /// Calculate current user statistics
    fn calculate_user_stats(&mut self) -> BbsResult<()> {
        let stats = self.services.users.get_stats()?;
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
    fn menu_handle_loop(&mut self, stream: &mut TelnetStream) -> BbsResult<bool> {
        // 1. Check user stats
        // This has to come first because of the mutable borrow
        let _ = self.calculate_user_stats();

        // 2. Get current menu and render
        let menu_render = {
            let menu_current = self.menu_get_current();
            menu_current.render(self)
        };

        // 3. Display menu (session handles I/O)
        self.menu_show(stream, &menu_render)?;

        // 4. Get input (session handles I/O) - now we can borrow mutably
        let input = self.get_input(stream, &menu_render.prompt)?;

        // 5. Handle input and process action
        let action = {
            let menu_current = self.menu_get_current();
            menu_current.handle_input(self, &input)
        };

        // 6. Process action (session handles state changes)
        self.menu_handle_action(stream, action)
    }

    /// Process menu actions and update session state
    fn menu_handle_action(
        &mut self,
        stream: &mut TelnetStream,
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
                self.menu_bulletin.state =
                    crate::menu::menu_bulletin::BulletinMenuState::PostingContent(title);
                Ok(true)
            }
            MenuAction::BulletinList => {
                let bulletins = self.get_all_bulletins()?;
                self.menu_bulletin.state =
                    crate::menu::menu_bulletin::BulletinMenuState::Listing(bulletins);
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

            // Message-specific actions
            MenuAction::MessageInbox => {
                let messages = self.get_user_inbox()?;
                self.menu_message.state =
                    crate::menu::menu_message::MessageMenuState::Inbox(messages);
                Ok(true)
            }
            MenuAction::MessageSent => {
                let messages = self.get_user_sent_messages()?;
                self.menu_message.state =
                    crate::menu::menu_message::MessageMenuState::Sent(messages);
                Ok(true)
            }
            MenuAction::MessageCompose => {
                self.menu_message.state = crate::menu::menu_message::MessageMenuState::Compose;
                Ok(true)
            }
            MenuAction::MessageComposeSubject(recipient) => {
                let subject = self.get_input(stream, "Subject: ")?;
                if subject.trim().is_empty() {
                    self.menu_message.state = crate::menu::menu_message::MessageMenuState::MainMenu;
                } else {
                    self.menu_message.state =
                        crate::menu::menu_message::MessageMenuState::ComposeContent {
                            recipient,
                            subject: subject.trim().to_string(),
                        };
                }
                Ok(true)
            }
            MenuAction::MessageSend {
                recipient,
                subject,
                content,
            } => {
                self.handle_message_send(stream, recipient, subject, content)?;
                Ok(true)
            }
            MenuAction::MessageRead(id) => {
                self.handle_message_read(stream, id)?;
                Ok(true)
            }
            MenuAction::MessageDelete(id) => {
                self.handle_message_delete(stream, id)?;
                Ok(true)
            }
            MenuAction::MessageBackToMenu => {
                self.menu_message.state = crate::menu::menu_message::MessageMenuState::MainMenu;
                Ok(true)
            }
        }
    }

    /// Get user input with a prompt - telnet handling now automatic via TelnetStream
    fn get_input(&mut self, stream: &mut TelnetStream, prompt: &str) -> BbsResult<String> {
        stream.queue(Print(prompt))?;
        stream.flush()?;

        let mut buffer = [0; 1024];
        match stream.read(&mut buffer) {
            Ok(0) => Err(BbsError::ClientDisconnected),
            Ok(n) => {
                // Phase 5: TelnetStream automatically handles all telnet processing
                // We only receive clean application data here
                let input = String::from_utf8_lossy(&buffer[0..n]);
                Ok(input.trim().to_string())
            }
            Err(e) => Err(BbsError::from(e)),
        }
    }

    /// Phase 7: Secure password input with echo control
    fn secure_password_input(
        &mut self,
        stream: &mut TelnetStream,
        prompt: &str,
    ) -> BbsResult<String> {
        // Disable echo for password security
        let _ = stream.request_echo_off()?;

        // Display prompt
        stream.queue(Print(prompt))?;
        stream.flush()?;

        let mut buffer = [0; 1024];
        let result = match stream.read(&mut buffer) {
            Ok(0) => Err(BbsError::ClientDisconnected),
            Ok(n) => {
                let input = String::from_utf8_lossy(&buffer[0..n]);
                Ok(input.trim().to_string())
            }
            Err(e) => Err(BbsError::from(e)),
        };

        // Re-enable echo after password input
        let _ = stream.request_echo_on()?;

        // Add a newline since echo was off
        stream.queue(Print("\n"))?;
        stream.flush()?;

        result
    }

    /// Initialize terminal state
    fn initialize_terminal(&mut self, stream: &mut TelnetStream) -> BbsResult<()> {
        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;
        stream.flush()?;
        Ok(())
    }

    /// Show the welcome screen
    fn show_welcome(&mut self, stream: &mut TelnetStream) -> BbsResult<()> {
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
            self.effective_width,
            Some(Color::Magenta),
        )?;

        stream.queue(Print("\nPress Enter to continue..."))?;
        stream.flush()?;

        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer);

        Ok(())
    }

    /// Handle user login process
    fn handle_login(&mut self, stream: &mut TelnetStream) -> BbsResult<()> {
        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;

        let instructions = "Choose an option:\n\n[L] Login with existing account\n[R] Register new account\n[C] Cancel";
        self.box_renderer.render_message_box(
            stream,
            "LOGIN / REGISTER",
            instructions,
            self.effective_width,
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
    fn handle_existing_login(&mut self, stream: &mut TelnetStream) -> BbsResult<()> {
        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;

        self.box_renderer.render_message_box(
            stream,
            "USER LOGIN",
            "Enter your credentials:",
            self.effective_width,
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

        let password = self.secure_password_input(stream, "Password: ")?;
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
        let registration_result = self.services.users.authenticate(&username, &password);

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
    fn handle_registration(&mut self, stream: &mut TelnetStream) -> BbsResult<()> {
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
            self.effective_width,
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
        let password = self.secure_password_input(stream, "Password (min 4 chars): ")?;
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
        let registration_result = self.services.users.register(request, &self.config);

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
    fn force_login(&mut self, stream: &mut TelnetStream) -> BbsResult<()> {
        let message = "This BBS requires registration to access. Anonymous access has been disabled by the SysOp.";

        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;

        self.box_renderer.render_message_box(
            stream,
            "LOGIN REQUIRED",
            message,
            self.effective_width,
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
    fn attempt_login(&mut self, stream: &mut TelnetStream) -> BbsResult<bool> {
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
            let password = self.secure_password_input(stream, "Password: ")?;

            // Try to authenticate
            match self.services.users.authenticate(&username, &password)? {
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
    fn menu_show(&self, stream: &mut TelnetStream, render: &MenuRender) -> BbsResult<()> {
        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;
        self.box_renderer.render_menu(
            stream,
            &render.title,
            &render.items,
            self.effective_width,
            None,
        )?;
        Ok(())
    }

    /// Display a message box with stream
    fn show_message_with_stream(
        &mut self,
        stream: &mut TelnetStream,
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
            self.effective_width,
            color,
        )?;

        stream.queue(Print("\nPress Enter to continue..."))?;
        stream.flush()?;

        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer);

        Ok(())
    }

    /// Show goodbye screen
    fn show_goodbye(&mut self, stream: &mut TelnetStream) -> BbsResult<()> {
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
            self.effective_width,
            Some(Color::Magenta),
        )?;

        stream.queue(Print("\nConnection will close in 3 seconds...\n"))?;
        stream.flush()?;

        std::thread::sleep(Duration::from_secs(3));
        Ok(())
    }

    /// Handle bulletin reading
    fn handle_bulletin_read(&mut self, stream: &mut TelnetStream, id: u32) -> BbsResult<()> {
        // Load bulletin from storage
        let bulletin = self.services.bulletins.get_bulletin(id)?;

        match bulletin {
            Some(bulletin) => {
                // Mark as read for logged-in users
                if let Some(user) = &self.user {
                    self.services.bulletins.mark_read(id, &user.username)?;
                }

                // Set menu to reading state
                self.menu_bulletin.state =
                    crate::menu::menu_bulletin::BulletinMenuState::Reading(bulletin.clone());

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
        stream: &mut TelnetStream,
        title: String,
        content: String,
    ) -> BbsResult<()> {
        let author = self.display_username();

        // Create bulletin request
        let request = crate::bulletins::BulletinRequest::new(title.clone(), content, author);

        // Post bulletin
        let result = self.services.bulletins.post_bulletin(request, &self.config);

        match result {
            Ok(bulletin_id) => {
                self.show_message_with_stream(
                    stream,
                    "BULLETIN POSTED",
                    &format!(
                        "Your bulletin '{}' has been posted as #{}",
                        title, bulletin_id
                    ),
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

        let stats = self.services.bulletins.get_stats(current_user)?;

        self.bulletin_stats = Some(stats);
        Ok(())
    }

    /// Refresh bulletin statistics
    fn get_all_bulletins(&mut self) -> BbsResult<Vec<Bulletin>> {
        // This method is not currently used, but keeping for potential future use
        // Would need to be implemented if needed
        todo!("get_all_bulletins not implemented for service layer")
    }

    /// Get user's inbox messages
    fn get_user_inbox(&self) -> BbsResult<Vec<crate::messages::PrivateMessage>> {
        if let Some(user) = &self.user {
            self.services.messages.get_inbox(&user.username)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get user's sent messages
    fn get_user_sent_messages(&self) -> BbsResult<Vec<crate::messages::PrivateMessage>> {
        if let Some(user) = &self.user {
            self.services.messages.get_sent(&user.username)
        } else {
            Ok(Vec::new())
        }
    }

    /// Handle sending a private message
    fn handle_message_send(
        &mut self,
        stream: &mut TelnetStream,
        recipient: String,
        subject: String,
        content: String,
    ) -> BbsResult<()> {
        let sender = self.display_username();

        if sender == "Anonymous" {
            self.show_message_with_stream(
                stream,
                "ERROR",
                "You must be logged in to send private messages.",
                Some(Color::Red),
            )?;
            return Ok(());
        }

        // Create message request
        let request = crate::messages::MessageRequest::new(
            recipient.clone(),
            subject.clone(),
            content,
            sender,
        );

        // Send message
        let result = self.services.messages.send_message(request, &self.config);

        match result {
            Ok(message_id) => {
                self.show_message_with_stream(
                    stream,
                    "MESSAGE SENT",
                    &format!(
                        "Your message '{}' has been sent to {} as #{}",
                        subject, recipient, message_id
                    ),
                    Some(Color::Green),
                )?;

                // Reset menu state
                self.menu_message.state = crate::menu::menu_message::MessageMenuState::MainMenu;
                Ok(())
            }
            Err(e) => {
                self.show_message_with_stream(
                    stream,
                    "SEND FAILED",
                    &format!("Failed to send message: {}", e),
                    Some(Color::Red),
                )?;
                Ok(())
            }
        }
    }

    /// Handle reading a private message
    fn handle_message_read(&mut self, stream: &mut TelnetStream, id: u32) -> BbsResult<()> {
        if let Some(user) = &self.user {
            match self.services.messages.read_message(id, &user.username)? {
                Some(message) => {
                    self.menu_message.state =
                        crate::menu::menu_message::MessageMenuState::Reading(message);
                    Ok(())
                }
                None => {
                    self.show_message_with_stream(
                        stream,
                        "MESSAGE NOT FOUND",
                        &format!(
                            "Message #{} was not found or you don't have permission to read it.",
                            id
                        ),
                        Some(Color::Red),
                    )?;
                    Ok(())
                }
            }
        } else {
            self.show_message_with_stream(
                stream,
                "ERROR",
                "You must be logged in to read private messages.",
                Some(Color::Red),
            )?;
            Ok(())
        }
    }

    /// Handle deleting a private message
    fn handle_message_delete(&mut self, stream: &mut TelnetStream, id: u32) -> BbsResult<()> {
        if let Some(user) = &self.user {
            match self.services.messages.delete_message(id, &user.username) {
                Ok(()) => {
                    self.show_message_with_stream(
                        stream,
                        "MESSAGE DELETED",
                        &format!("Message #{} has been deleted.", id),
                        Some(Color::Green),
                    )?;
                    // Return to inbox
                    self.menu_message.state = crate::menu::menu_message::MessageMenuState::MainMenu;
                    Ok(())
                }
                Err(e) => {
                    self.show_message_with_stream(
                        stream,
                        "DELETE FAILED",
                        &format!("Failed to delete message: {}", e),
                        Some(Color::Red),
                    )?;
                    Ok(())
                }
            }
        } else {
            self.show_message_with_stream(
                stream,
                "ERROR",
                "You must be logged in to delete private messages.",
                Some(Color::Red),
            )?;
            Ok(())
        }
    }

    // Show feature disabled message
    // fn show_feature_disabled(
    //     &mut self,
    //     stream: &mut TelnetStream,
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
