use crate::box_renderer::{BoxRenderer, BoxStyle};
use crate::config::BbsConfig;
use crate::errors::{BbsError, BbsResult};
use crate::menu::{CurrentMenu, Menu, MenuAction, MenuData, MenuRender};
use crossterm::{
    cursor,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
    QueueableCommand,
};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

pub struct BbsSession {
    // All session state in one place
    pub config: Arc<BbsConfig>,
    pub username: Option<String>,
    pub menu_current: CurrentMenu,

    // Session resources
    box_renderer: BoxRenderer,
    login_attempts: u8,

    // Menu instances (owned by session, can maintain state)
    menu_main: crate::menu::menu_main::MainMenu,
    menu_bulletin: crate::menu::menu_bulletin::BulletinMenu,
    // menu_user: crate::menu::menu_user::UserMenu,
    // menu_message: crate::menu::menu_message::MessageMenu,
    // menu_file: crate::menu::menu_file::FileMenu,
}

impl BbsSession {
    pub fn new(config: Arc<BbsConfig>) -> Self {
        let box_renderer = BoxRenderer::new(BoxStyle::Ascii);

        Self {
            config,
            username: None,
            menu_current: CurrentMenu::Main,
            box_renderer,
            login_attempts: 0,
            menu_main: crate::menu::menu_main::MainMenu::new(),
            menu_bulletin: crate::menu::menu_bulletin::BulletinMenu::new(),
            // menu_user: crate::menu::menu_user::UserMenu::new(),
            // menu_message: crate::menu::menu_message::MessageMenu::new(),
            // menu_file: crate::menu::menu_file::FileMenu::new(),
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
        if !self.config.features.allow_anonymous && self.username.is_none() {
            self.force_login(&mut stream)?;
        }

        // Main session loop
        loop {
            if !self.menu_handle_loop(&mut stream)? {
                break; // User chose to quit
            }
        }

        Ok(())
    }

    /// Get the current menu instance
    fn menu_get_current(&self) -> &dyn Menu {
        match self.menu_current {
            CurrentMenu::Main => &self.menu_main,
            CurrentMenu::Bulletins => &self.menu_bulletin,
            // CurrentMenu::Users => &self.menu_user,
            // CurrentMenu::Messages => &self.menu_message,
            // CurrentMenu::Files => &self.menu_file,
        }
    }

    /// Create MenuData for current session state
    fn menu_create_data(&self) -> MenuData<'_> {
        MenuData {
            config: &self.config,
            username: &self.username,
        }
    }

    /// Main menu loop - render, display, get input, handle action
    fn menu_handle_loop(&mut self, stream: &mut TcpStream) -> BbsResult<bool> {
        // 1. Get current menu
        let menu_current = self.menu_get_current();

        // 2. Create menu data
        let menu_data = self.menu_create_data();

        // 3. Render menu (pure function, returns data)
        let menu_render = menu_current.render(menu_data);

        // 4. Display menu (session handles I/O)
        self.menu_show(stream, &menu_render)?;

        // 5. Get input (session handles I/O)
        let input = self.get_input(stream, &menu_render.prompt)?;

        // 6. Handle input (pure function, returns action)
        let action = menu_current.handle_input(menu_data, &input);

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
                self.username = None;
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
        stream.queue(SetForegroundColor(Color::Cyan))?;

        let welcome = self.config.get_welcome_header();
        stream.queue(Print(welcome))?;
        stream.queue(ResetColor)?;
        stream.flush()?;

        std::thread::sleep(Duration::from_millis(self.config.ui.welcome_pause_ms));
        Ok(())
    }

    /// Handle user login process
    fn handle_login(&mut self, stream: &mut TcpStream) -> BbsResult<()> {
        let width = self.config.ui.menu_width;

        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;

        let instructions = format!(
            "Max username length: {} characters\n\nEnter username (or press Enter to cancel):",
            self.config.features.max_username_length
        );

        self.box_renderer.render_message_box(
            stream,
            "USER LOGIN",
            &instructions,
            width,
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

        if username.len() > self.config.features.max_username_length {
            let msg = format!(
                "Username too long (max {} characters)",
                self.config.features.max_username_length
            );
            return Err(BbsError::InvalidInput(msg));
        }

        self.username = Some(username.clone());
        let welcome_msg = format!("Welcome to {}, {}!", self.config.bbs.name, username);
        self.show_message_with_stream(stream, "WELCOME", &welcome_msg, Some(Color::Green))?;

        Ok(())
    }

    /// Force login for restricted BBS
    fn force_login(&mut self, stream: &mut TcpStream) -> BbsResult<()> {
        let width = self.config.ui.menu_width + 20;
        let message = "This BBS requires registration to access. Anonymous access has been disabled by the SysOp.";

        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;

        self.box_renderer.render_message_box(
            stream,
            "LOGIN REQUIRED",
            message,
            width,
            Some(Color::Yellow),
        )?;

        let timeout = self.config.timeouts.login_timeout;
        stream.set_read_timeout(Some(timeout))?;

        while self.username.is_none() && self.login_attempts < 3 {
            if !self.attempt_login(stream)? {
                break;
            }
        }

        if self.username.is_none() {
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
            self.username = Some(username.clone());
            stream.queue(SetForegroundColor(Color::Green))?;
            stream.queue(Print(&format!("Welcome, {}!\n\n", username)))?;
            stream.queue(ResetColor)?;
            stream.flush()?;
            std::thread::sleep(Duration::from_secs(1));
            return Ok(false);
        }

        Ok(true)
    }

    /// Display a rendered menu
    fn menu_show(&self, stream: &mut TcpStream, render: &MenuRender) -> BbsResult<()> {
        let width = self.config.ui.menu_width;
        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;
        self.box_renderer
            .render_menu(stream, &render.title, &render.items, width, None)?;
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
        let width = self.config.ui.menu_width + 20;

        stream.queue(Clear(ClearType::All))?;
        stream.queue(cursor::MoveTo(0, 0))?;

        self.box_renderer
            .render_message_box(stream, title, message, width, color)?;

        stream.queue(Print("\nPress Enter to continue..."))?;
        stream.flush()?;

        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer);

        Ok(())
    }

    /// Show goodbye screen
    fn show_goodbye(&mut self, stream: &mut TcpStream) -> BbsResult<()> {
        let width = self.config.ui.menu_width + 20;

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
            width,
            Some(Color::Magenta),
        )?;

        stream.queue(Print("\nConnection will close in 3 seconds...\n"))?;
        stream.flush()?;

        std::thread::sleep(Duration::from_secs(3));
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

// Remove the problematic Display trait implementation
// impl Display for BbsSession {
