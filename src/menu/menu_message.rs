use crate::box_renderer::MenuItem;
use crate::menu::{MenuAction, MenuRender, MenuScreen};
use crate::messages::PrivateMessage;
use crate::session::BbsSession;

pub struct MessageMenu {
    pub state: MessageMenuState,
}

#[derive(Debug, Clone)]
pub enum MessageMenuState {
    MainMenu,
    Inbox(Vec<PrivateMessage>),
    Sent(Vec<PrivateMessage>),
    Compose,
    ComposeContent { recipient: String, subject: String },
    Reading(PrivateMessage),
}

impl MessageMenu {
    pub fn new() -> Self {
        Self {
            state: MessageMenuState::MainMenu,
        }
    }
}

impl MenuScreen for MessageMenu {
    fn render(&self, session: &BbsSession) -> MenuRender {
        match &self.state {
            MessageMenuState::MainMenu => render_main_menu(session),
            MessageMenuState::Inbox(messages) => render_inbox(session, messages),
            MessageMenuState::Sent(messages) => render_sent(session, messages),
            MessageMenuState::Compose => render_compose(session),
            MessageMenuState::ComposeContent { recipient, subject } => {
                render_compose_content(session, recipient, subject)
            }
            MessageMenuState::Reading(message) => render_message(session, message),
        }
    }

    fn handle_input(&self, session: &BbsSession, input: &str) -> MenuAction {
        match &self.state {
            MessageMenuState::MainMenu => handle_main_menu_input(session, input),
            MessageMenuState::Inbox(messages) => handle_inbox_input(session, input, messages),
            MessageMenuState::Sent(messages) => handle_sent_input(session, input, messages),
            MessageMenuState::Compose => handle_compose_input(session, input),
            MessageMenuState::ComposeContent { recipient, subject } => {
                handle_compose_content_input(session, input, recipient, subject)
            }
            MessageMenuState::Reading(_message) => handle_reading_input(session, input),
        }
    }
}

fn render_main_menu(session: &BbsSession) -> MenuRender {
    let username = session.display_username();
    let title = format!("PRIVATE MESSAGES - {}", username);

    let mut items = vec![];

    // Get message stats if user is logged in
    if let Some(user) = &session.user {
        if let Ok(stats) = session.services.messages.get_stats(&user.username) {
            items.extend([
                MenuItem::info(&format!("Unread Messages: {}", stats.unread_count)),
                MenuItem::info(&format!("Total Received: {}", stats.total_received)),
                MenuItem::info(&format!("Total Sent: {}", stats.total_sent)),
                MenuItem::blank(),
            ]);
        }
    }

    items.extend([
        MenuItem::option("I", "Inbox"),
        MenuItem::option("S", "Sent Messages"),
        MenuItem::option("C", "Compose New Message"),
        MenuItem::blank(),
        MenuItem::option("M", "Main Menu"),
        MenuItem::option("Q", "Quit"),
    ]);

    MenuRender::with_items(&title, items, "Choice: ")
}

fn render_inbox(session: &BbsSession, messages: &[PrivateMessage]) -> MenuRender {
    let username = session.display_username();
    let title = format!("INBOX - {} ({} messages)", username, messages.len());

    let mut items = vec![];

    if messages.is_empty() {
        items.extend([
            MenuItem::info("No messages in your inbox."),
            MenuItem::blank(),
        ]);
    } else {
        items.push(MenuItem::info(
            // Leading spaces for status
            "    ID | From         | Subject                | Sent",
        ));
        items.push(MenuItem::separator());

        for (index, message) in messages.iter().enumerate().take(20) {
            let status = if message.is_unread() { "[N]" } else { "   " };
            let from_truncated = if message.sender.len() > 12 {
                format!("{}...", &message.sender[..9])
            } else {
                format!("{:12}", message.sender)
            };
            let subject_truncated = if message.subject.len() > 22 {
                format!("{}...", &message.subject[..19])
            } else {
                format!("{:22}", message.subject)
            };

            items.push(MenuItem::info(&format!(
                "{} {:2} | {} | {} | {}",
                status,
                index + 1,
                from_truncated,
                subject_truncated,
                message.sent_display()
            )));
        }

        items.extend([
            MenuItem::blank(),
            MenuItem::info("Enter message number to read, or:"),
        ]);
    }

    items.extend([
        MenuItem::blank(),
        MenuItem::option("C", "Compose New Message"),
        MenuItem::option("R", "Refresh Inbox"),
        MenuItem::option("B", "Back to Message Menu"),
        MenuItem::option("M", "Main Menu"),
    ]);

    MenuRender::with_items(&title, items, "Choice: ")
}

fn render_sent(session: &BbsSession, messages: &[PrivateMessage]) -> MenuRender {
    let username = session.display_username();
    let title = format!("SENT MESSAGES - {} ({} messages)", username, messages.len());

    let mut items = vec![];

    if messages.is_empty() {
        items.extend([
            MenuItem::info("You haven't sent any messages yet."),
            MenuItem::blank(),
        ]);
    } else {
        items.push(MenuItem::info(
            // Leading spaces for status
            "    ID | To           | Subject                | Sent",
        ));
        items.push(MenuItem::separator());

        for (index, message) in messages.iter().enumerate().take(20) {
            let read_status = if message.is_unread() { "   " } else { "[R]" };
            let to_truncated = if message.recipient.len() > 12 {
                format!("{}...", &message.recipient[..9])
            } else {
                format!("{:12}", message.recipient)
            };
            let subject_truncated = if message.subject.len() > 22 {
                format!("{}...", &message.subject[..19])
            } else {
                format!("{:22}", message.subject)
            };

            items.push(MenuItem::info(&format!(
                "{} {:2} | {} | {} | {}",
                read_status,
                index + 1,
                to_truncated,
                subject_truncated,
                message.sent_display()
            )));
        }

        items.extend([
            MenuItem::blank(),
            MenuItem::info("Enter message number to view, or:"),
        ]);
    }

    items.extend([
        MenuItem::blank(),
        MenuItem::option("C", "Compose New Message"),
        MenuItem::option("R", "Refresh Sent"),
        MenuItem::option("B", "Back to Message Menu"),
        MenuItem::option("M", "Main Menu"),
    ]);

    MenuRender::with_items(&title, items, "Choice: ")
}

fn render_compose(_session: &BbsSession) -> MenuRender {
    let title = "COMPOSE MESSAGE";

    let items = vec![
        MenuItem::info("Enter the username of the recipient."),
        MenuItem::info("Leave blank to cancel."),
        MenuItem::blank(),
    ];

    MenuRender::with_items(title, items, "Recipient: ")
}

fn render_compose_content(_session: &BbsSession, recipient: &str, subject: &str) -> MenuRender {
    let title = "COMPOSE MESSAGE";

    let items = vec![
        MenuItem::info(&format!("To: {}", recipient)),
        MenuItem::info(&format!("Subject: {}", subject)),
        MenuItem::blank(),
        MenuItem::info("Enter your message content:"),
        MenuItem::info("(Leave blank to cancel)"),
        MenuItem::blank(),
    ];

    MenuRender::with_items(title, items, "Message: ")
}

fn render_message(session: &BbsSession, message: &PrivateMessage) -> MenuRender {
    let title = "READING MESSAGE";

    let mut items = vec![
        MenuItem::info(&format!("From: {}", message.sender)),
        MenuItem::info(&format!("To: {}", message.recipient)),
        MenuItem::info(&format!("Subject: {}", message.subject)),
        MenuItem::info(&format!("Sent: {}", message.sent_display())),
        MenuItem::separator(),
    ];

    // Add message content, wrapped to fit menu width
    let content_width = session.config.ui.menu_width.saturating_sub(4);
    let content_lines = message.get_content_lines(content_width);

    for line in content_lines {
        items.push(MenuItem::info(&line));
    }

    items.extend([
        MenuItem::separator(),
        MenuItem::blank(),
        MenuItem::option("R", "Reply"),
        MenuItem::option("D", "Delete"),
        MenuItem::option("B", "Back to Inbox"),
        MenuItem::option("M", "Main Menu"),
    ]);

    MenuRender::with_items(title, items, "Choice: ")
}

fn handle_main_menu_input(_session: &BbsSession, input: &str) -> MenuAction {
    match input.to_lowercase().as_str() {
        "i" | "inbox" => MenuAction::MessageInbox,
        "s" | "sent" => MenuAction::MessageSent,
        "c" | "compose" => MenuAction::MessageCompose,
        "m" | "main" => MenuAction::GoTo(crate::menu::Menu::Main),
        "q" | "quit" => MenuAction::Quit,
        _ => MenuAction::ShowMessage("Invalid choice. Please try again.".to_string()),
    }
}

fn handle_inbox_input(
    _session: &BbsSession,
    input: &str,
    messages: &[PrivateMessage],
) -> MenuAction {
    match input.to_lowercase().as_str() {
        "c" | "compose" => MenuAction::MessageCompose,
        "r" | "refresh" => MenuAction::MessageInbox,
        "b" | "back" => MenuAction::MessageBackToMenu,
        "m" | "main" => MenuAction::GoTo(crate::menu::Menu::Main),
        _ => {
            // Try to parse as message number
            if let Ok(num) = input.parse::<usize>() {
                if num > 0 && num <= messages.len() {
                    let message = &messages[num - 1];
                    MenuAction::MessageRead(message.id)
                } else {
                    MenuAction::ShowMessage("Invalid message number.".to_string())
                }
            } else {
                MenuAction::ShowMessage(
                    "Invalid choice. Enter a message number or command.".to_string(),
                )
            }
        }
    }
}

fn handle_sent_input(
    _session: &BbsSession,
    input: &str,
    messages: &[PrivateMessage],
) -> MenuAction {
    match input.to_lowercase().as_str() {
        "c" | "compose" => MenuAction::MessageCompose,
        "r" | "refresh" => MenuAction::MessageSent,
        "b" | "back" => MenuAction::MessageBackToMenu,
        "m" | "main" => MenuAction::GoTo(crate::menu::Menu::Main),
        _ => {
            // Try to parse as message number
            if let Ok(num) = input.parse::<usize>() {
                if num > 0 && num <= messages.len() {
                    let message = &messages[num - 1];
                    MenuAction::MessageRead(message.id)
                } else {
                    MenuAction::ShowMessage("Invalid message number.".to_string())
                }
            } else {
                MenuAction::ShowMessage(
                    "Invalid choice. Enter a message number or command.".to_string(),
                )
            }
        }
    }
}

fn handle_compose_input(_session: &BbsSession, input: &str) -> MenuAction {
    if input.trim().is_empty() {
        MenuAction::MessageBackToMenu
    } else {
        MenuAction::MessageComposeSubject(input.trim().to_string())
    }
}

fn handle_compose_content_input(
    _session: &BbsSession,
    input: &str,
    recipient: &str,
    subject: &str,
) -> MenuAction {
    if input.trim().is_empty() {
        MenuAction::MessageBackToMenu
    } else {
        MenuAction::MessageSend {
            recipient: recipient.to_string(),
            subject: subject.to_string(),
            content: input.trim().to_string(),
        }
    }
}

fn handle_reading_input(_session: &BbsSession, input: &str) -> MenuAction {
    match input.to_lowercase().as_str() {
        "r" | "reply" => {
            // TODO: Implement reply functionality
            MenuAction::ShowMessage("Reply feature coming soon!".to_string())
        }
        "d" | "delete" => {
            // TODO: Get message ID from current state
            MenuAction::ShowMessage("Delete feature coming soon!".to_string())
        }
        "b" | "back" => MenuAction::MessageInbox,
        "m" | "main" => MenuAction::GoTo(crate::menu::Menu::Main),
        _ => MenuAction::ShowMessage("Invalid choice. Please try again.".to_string()),
    }
}
