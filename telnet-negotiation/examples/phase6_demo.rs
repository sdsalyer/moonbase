//! # Phase 6 Demo: Specific Telnet Options
//!
//! This demo showcases the Phase 6 implementation of specific telnet options:
//! - Echo Option (RFC 857) for secure password input
//! - Terminal Type Option (RFC 1091) for capability detection  
//! - NAWS Option (RFC 1073) for window size negotiation
//! - High-level API for BBS integration
//!
//! ## What Phase 6 Delivers
//!
//! Phase 6 completes the telnet option negotiation foundation needed for
//! Phase 7's Enhanced BBS Experience by implementing:
//!
//! 1. **Core Option Implementations**: Echo, Terminal Type, NAWS
//! 2. **Sub-negotiation Support**: Full RFC-compliant sub-negotiation processing
//! 3. **Option Handler Registry**: Extensible framework for custom options
//! 4. **High-level API**: Convenient methods for common BBS use cases
//!
//! ## Demo Features
//!
//! - Interactive telnet client simulation
//! - All three core options demonstrated
//! - Sub-negotiation message examples  
//! - High-level API usage patterns
//! - Integration readiness for Phase 7

use std::io;
use telnet_negotiation::options::TelnetOptionHandler;
use telnet_negotiation::{
    ColorDepth, EchoOption, EchoState, NawsOption, TerminalCapabilities, TerminalTypeOption,
    WindowSize,
};

fn main() -> io::Result<()> {
    println!("Phase 6 Demo: Specific Telnet Options");
    println!("=====================================");
    println!();

    // Demo the three core options implemented in Phase 6
    demo_echo_option();
    demo_terminal_type_option();
    demo_naws_option();
    demo_integration_capabilities();

    println!("Phase 6 Complete: Ready for Phase 7 Enhanced BBS Experience!");
    println!();
    println!("Next: Phase 7 will use these options to provide:");
    println!("   - Secure password input via Echo control");
    println!("   - Responsive layouts via NAWS window detection");
    println!("   - Adaptive rendering via Terminal Type capabilities");
    println!("   - Smart color themes based on terminal support");

    Ok(())
}

fn demo_echo_option() {
    println!("Echo Option Demo (RFC 857)");
    println!("--------------------------");

    // Create echo option handlers for both server and client
    let mut server_echo = EchoOption::new(true); // Server-side
    let client_echo = EchoOption::new(false); // Client-side

    println!("Initial state:");
    println!("  Server echo state: {:?}", server_echo.state());
    println!("  Client echo state: {:?}", client_echo.state());
    println!();

    // Simulate password input scenario
    println!("Password Input Scenario:");
    println!("  1. Server wants to handle echoing (for security)");
    server_echo.enable_remote_echo();
    println!("     Server sets remote echo: {:?}", server_echo.state());

    println!("  2. Server would send: IAC WILL ECHO");
    let should_send_will = server_echo.should_send_will(EchoState::RemoteEcho);
    println!("     Should server send WILL? {}", should_send_will);

    println!("  3. Characters typed by user are not echoed locally");
    println!(
        "     Echo state check: is_remote_echo = {}",
        server_echo.is_remote_echo()
    );

    println!("  4. After password input, restore normal echoing");
    server_echo.enable_local_echo();
    println!("     Server restores local echo: {:?}", server_echo.state());
    println!("     Server would send: IAC WONT ECHO");

    println!();
}

fn demo_terminal_type_option() {
    println!("Terminal Type Option Demo (RFC 1091)");
    println!("------------------------------------");

    let mut terminal_type = TerminalTypeOption::new();

    println!("Initial state:");
    println!("  Has terminal type data: {}", terminal_type.is_active());
    println!("  Terminal type: {:?}", terminal_type.terminal_type());
    println!();

    // Simulate receiving terminal type information
    println!("Sub-negotiation Example:");
    println!("  Client sends: IAC SB TERMINAL_TYPE IS \"XTERM-256COLOR\" IAC SE");

    // Simulate the IS sub-negotiation data (command byte 1 + terminal type string)
    let mut subneg_data = vec![1]; // IS command (1)
    subneg_data.extend_from_slice(b"XTERM-256COLOR");
    let result = terminal_type.handle_subnegotiation(&subneg_data);

    match result {
        Ok(_) => {
            println!("  Terminal type negotiation successful!");
            println!(
                "  Terminal type: {:?}",
                terminal_type.terminal_type().unwrap_or("Unknown")
            );

            if let Some(caps) = terminal_type.capabilities() {
                println!("  Detected capabilities:");
                println!("    - ANSI support: {}", caps.ansi_support);
                println!("    - Color support: {:?}", caps.color_support);
                println!("    - Cursor positioning: {}", caps.cursor_positioning);
                println!("    - Alternate screen: {}", caps.alternate_screen);
            }
        }
        Err(e) => println!("  Error: {}", e),
    }

    println!();

    // Test different terminal types and their capabilities
    println!("Capability Detection Examples:");
    let test_terminals = [
        "VT100",
        "ANSI",
        "XTERM",
        "XTERM-256COLOR",
        "SCREEN-256COLOR",
    ];

    for &term_type in &test_terminals {
        let mut test_terminal = TerminalTypeOption::new();
        test_terminal.set_terminal_type(term_type.to_string());

        if let Some(caps) = test_terminal.capabilities() {
            println!(
                "  {} -> ANSI: {}, Colors: {:?}",
                term_type, caps.ansi_support, caps.color_support
            );
        }
    }

    println!();
}

fn demo_naws_option() {
    println!("NAWS Option Demo (RFC 1073)");
    println!("---------------------------");

    let mut naws = NawsOption::new();

    println!("Initial state:");
    println!("  Has window size data: {}", naws.has_size_data());
    println!("  Window size: {:?}", naws.window_size());
    println!();

    // Simulate receiving window size information
    println!("Sub-negotiation Example:");
    println!("  Client sends: IAC SB NAWS <width> <height> IAC SE");

    // Simulate NAWS sub-negotiation data: 80x24 terminal
    // Format: <width-high> <width-low> <height-high> <height-low>
    let subneg_data = [0x00, 0x50, 0x00, 0x18]; // 80 (0x0050) x 24 (0x0018)
    let result = naws.handle_subnegotiation(&subneg_data);

    match result {
        Ok(_) => {
            println!("  Window size negotiation successful!");
            if let Some(size) = naws.window_size() {
                println!("  Window size: {}", size); // Uses Display trait
                println!("  Width: {} characters", size.width);
                println!("  Height: {} lines", size.height);
                println!("  Total capacity: {} characters", size.capacity());
                println!("  Is reasonable size: {}", size.is_reasonable());
            }
        }
        Err(e) => println!("  Error: {}", e),
    }

    println!();

    // Test different window sizes
    println!("Window Size Examples:");
    let test_sizes = [
        (80, 24),  // Standard terminal
        (132, 43), // Wide terminal
        (120, 30), // Modern terminal
        (0, 0),    // Unknown size
    ];

    for &(width, height) in &test_sizes {
        let size = WindowSize::new(width, height);
        println!(
            "  {}x{} -> Valid: {}, Reasonable: {}",
            width,
            height,
            size.is_valid(),
            size.is_reasonable()
        );
    }

    println!();
}

fn demo_integration_capabilities() {
    println!("Integration & High-level API Demo");
    println!("---------------------------------");

    // This would be used in a real BBS application
    println!("Capabilities Structure for BBS Integration:");

    // Simulate a fully negotiated terminal session
    let mut caps = TerminalCapabilities::default();
    caps.width = Some(80);
    caps.height = Some(24);
    caps.terminal_type = Some("XTERM-256COLOR".to_string());
    caps.supports_ansi = true;
    caps.supports_color = true;
    caps.color_depth = ColorDepth::Extended256;

    println!("  Terminal: {:?}", caps.terminal_type);
    println!(
        "  Dimensions: {}x{}",
        caps.width.unwrap_or(0),
        caps.height.unwrap_or(0)
    );
    println!("  ANSI Support: {}", caps.supports_ansi);
    println!(
        "  Color Support: {} ({:?})",
        caps.supports_color, caps.color_depth
    );

    println!();
    println!("BBS Usage Examples:");
    println!("  if caps.supports_color && matches!(caps.color_depth, ColorDepth::Extended256) {{");
    println!("      // Use 256-color themes");
    println!("  }}");
    println!();
    println!("  if let Some(width) = caps.width {{");
    println!("      // Responsive menu layout based on terminal width");
    println!("  }}");
    println!();
    println!("  // Secure password input:");
    println!("  stream.request_echo_off()?;");
    println!("  let password = get_password_input(&mut stream)?;");
    println!("  stream.request_echo_on()?;");

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_option_behavior() {
        let mut echo = EchoOption::new(true);

        // Test password scenario
        echo.enable_remote_echo();
        assert!(echo.is_remote_echo());
        assert!(echo.should_send_will(EchoState::RemoteEcho));

        // Test normal input restoration
        echo.enable_local_echo();
        assert!(echo.is_local_echo());
        assert!(!echo.should_send_will(EchoState::LocalEcho));
    }

    #[test]
    fn test_terminal_type_capabilities() {
        let mut term_type = TerminalTypeOption::new();

        // Test modern terminal
        term_type.set_terminal_type("XTERM-256COLOR".to_string());

        assert!(term_type.is_active());
        assert_eq!(term_type.terminal_type(), Some("XTERM-256COLOR"));
        assert!(term_type.supports_ansi());
        assert!(term_type.supports_color());

        let caps = term_type.capabilities().unwrap();
        assert!(caps.ansi_support);
        assert!(caps.cursor_positioning);
        assert!(caps.alternate_screen);
    }

    #[test]
    fn test_naws_window_size() {
        let mut naws = NawsOption::new();

        // Test standard terminal size
        naws.set_window_size(80, 24);

        assert!(naws.has_size_data());
        assert!(naws.is_active());
        assert_eq!(naws.width(), Some(80));
        assert_eq!(naws.height(), Some(24));

        let size = naws.window_size().unwrap();
        assert!(size.is_valid());
        assert!(size.is_reasonable());
        assert_eq!(size.capacity(), 1920);
    }

    #[test]
    fn test_terminal_capabilities_integration() {
        let caps = TerminalCapabilities {
            width: Some(132),
            height: Some(43),
            terminal_type: Some("XTERM".to_string()),
            supports_ansi: true,
            supports_color: true,
            color_depth: ColorDepth::Basic8,
        };

        // Test BBS decision making
        assert!(caps.width.unwrap() > 80); // Wide terminal
        assert!(caps.supports_color); // Can use colors
        assert!(matches!(caps.color_depth, ColorDepth::Basic8)); // But limited palette
    }
}
