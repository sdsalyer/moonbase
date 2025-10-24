//! # Phase 2 Demo: Protocol Constants and Types
//!
//! This example demonstrates the Phase 2 functionality of the telnet-negotiation library.
//! Run with: `cargo run --example phase2_demo`

use telnet_negotiation::{IAC, TelnetCommand, TelnetOption, TelnetSequence};

fn main() {
    println!("=== Telnet Negotiation Library - Phase 2 Demo ===\n");

    // Demonstrate IAC constant
    println!("1. IAC (Interpret As Command) constant:");
    println!("   IAC = {} (0x{:02X})", IAC, IAC);

    // Demonstrate command parsing
    println!("\n2. Command byte parsing:");
    let will_byte = 251;
    match TelnetCommand::from_byte(will_byte) {
        Some(cmd) => println!("   Byte {} -> {:?}", will_byte, cmd),
        None => println!("   Byte {} -> Unknown command", will_byte),
    }

    // Demonstrate option parsing
    println!("\n3. Option byte parsing:");
    let echo_byte = 1;
    match TelnetOption::from_byte(echo_byte) {
        Some(opt) => {
            println!("   Byte {} -> {:?}", echo_byte, opt);
            if let Some(rfc) = opt.rfc_number() {
                println!("   Defined in RFC {}", rfc);
            }
        }
        None => println!("   Byte {} -> Unknown option", echo_byte),
    }

    // Demonstrate MUD/MUSH extensions
    println!("\n4. MUD/MUSH protocol extensions:");
    let mud_options = [TelnetOption::GMCP, TelnetOption::MCCP2, TelnetOption::MXP];

    for option in mud_options {
        println!(
            "   {:?} ({}): MUD extension = {}",
            option,
            option.to_byte(),
            option.is_mud_extension()
        );
    }

    // Demonstrate sequence serialization
    println!("\n5. Telnet sequence serialization:");

    // Simple command: IAC NOP
    let nop_cmd = TelnetSequence::Command(TelnetCommand::NOP);
    println!("   IAC NOP -> {:?}", nop_cmd.to_bytes());

    // Negotiation: IAC WILL ECHO
    let will_echo = TelnetSequence::Negotiation {
        command: TelnetCommand::WILL,
        option: TelnetOption::ECHO,
    };
    println!("   IAC WILL ECHO -> {:?}", will_echo.to_bytes());

    // Sub-negotiation: IAC SB TERMINAL_TYPE SEND IAC SE
    let terminal_type_query = TelnetSequence::SubNegotiation {
        option: TelnetOption::TERMINAL_TYPE,
        data: vec![1], // SEND command for terminal type
    };
    println!(
        "   IAC SB TERMINAL_TYPE SEND IAC SE -> {:?}",
        terminal_type_query.to_bytes()
    );

    // Escaped data: data byte 255 -> IAC IAC
    let escaped_data = TelnetSequence::EscapedData(255);
    println!(
        "   Data byte 255 (escaped) -> {:?}",
        escaped_data.to_bytes()
    );

    println!("\n6. Command categorization:");
    let commands = [
        TelnetCommand::WILL,
        TelnetCommand::NOP,
        TelnetCommand::SB,
        TelnetCommand::AYT,
    ];

    for cmd in commands {
        println!(
            "   {:?}: negotiation={}, requires_option={}",
            cmd,
            cmd.is_negotiation_command(),
            cmd.requires_option()
        );
    }

    println!("\n=== Phase 2 Demo Complete ===");
    println!("This demonstrates the foundation for Telnet protocol handling.");
    println!("Next phases will add parsing, negotiation state machines, and stream integration.");
}
