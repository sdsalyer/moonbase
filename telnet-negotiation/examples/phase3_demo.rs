//! # Phase 3 Demo: Command Detection and Parsing
//!
//! This example demonstrates the Phase 3 functionality: parsing Telnet commands
//! from byte streams and separating data from protocol commands.
//! Run with: `cargo run --example phase3_demo`

use telnet_negotiation::{TelnetCommand, TelnetOption, TelnetParser, TelnetSequence};

fn main() {
    println!("=== Telnet Negotiation Library - Phase 3 Demo ===\n");

    let mut parser = TelnetParser::new();

    println!("1. Parsing simple data (no telnet commands):");
    let simple_data = b"Hello, BBS World!";
    let result = parser.parse(simple_data);
    println!("   Input: {:?}", String::from_utf8_lossy(simple_data));
    println!("   Data: {:?}", String::from_utf8_lossy(&result.data));
    println!("   Commands: {} detected", result.sequences.len());
    println!("   Bytes consumed: {}", result.bytes_consumed);

    println!("\n2. Parsing data with embedded telnet commands:");
    let mixed_data = vec![
        // "login: "
        108, 111, 103, 105, 110, 58, 32, // IAC WILL ECHO (server will echo)
        255, 251, 1, // "password: "
        112, 97, 115, 115, 119, 111, 114, 100, 58, 32,
        // IAC WONT ECHO (server stops echoing for password)
        255, 252, 1,
    ];

    let result = parser.parse(&mixed_data);
    println!("   Input length: {} bytes", mixed_data.len());
    println!("   Data: {:?}", String::from_utf8_lossy(&result.data));
    println!("   Commands detected: {}", result.sequences.len());

    for (i, seq) in result.sequences.iter().enumerate() {
        match seq {
            TelnetSequence::Negotiation { command, option } => {
                println!("     Command {}: {:?} {:?}", i + 1, command, option);
            }
            TelnetSequence::Command(cmd) => {
                println!("     Command {}: {:?}", i + 1, cmd);
            }
            TelnetSequence::EscapedData(byte) => {
                println!("     Command {}: Escaped data byte {}", i + 1, byte);
            }
            TelnetSequence::SubNegotiation { option, data } => {
                println!(
                    "     Command {}: Sub-negotiation {:?} with {} bytes",
                    i + 1,
                    option,
                    data.len()
                );
            }
        }
    }

    println!("\n3. Testing IAC escaping (data byte 255):");
    let escaped_data = vec![
        72, 101, 108, 108, 111, // "Hello"
        255, 255, // IAC IAC (escaped 255)
        87, 111, 114, 108, 100, // "World"
    ];

    let result = parser.parse(&escaped_data);
    println!("   Raw data bytes: {:?}", result.data);
    println!(
        "   As UTF-8 string: {:?}",
        String::from_utf8_lossy(&result.data)
    );
    println!(
        "   Commands: {} (should be 1 for the escaped byte)",
        result.sequences.len()
    );
    println!("   Explanation: IAC IAC (255,255) becomes literal byte 255 in data");
    println!("                Byte 255 is not valid UTF-8, so displays as replacement char �");

    println!("\n4. Testing sub-negotiation (Terminal Type query):");
    let terminal_negotiation = vec![
        255, 250, 24, 1, 255, 240, // IAC SB TERMINAL_TYPE SEND IAC SE
    ];

    let result = parser.parse(&terminal_negotiation);
    println!("   Commands: {}", result.sequences.len());
    if let Some(TelnetSequence::SubNegotiation { option, data }) = result.sequences.first() {
        println!("     Option: {:?} (RFC 1091)", option);
        println!("     Sub-data: {:?}", data);
        println!(
            "     Interpretation: SEND command (byte 1 = 'Please tell me your terminal type')"
        );
        println!("     Expected response: IAC SB TERMINAL_TYPE IS \"ANSI\" IAC SE");
        println!("                        (where IS = byte 0, followed by terminal name)");
    }

    println!("\n   Example of a terminal type response:");
    let terminal_response = vec![
        255, 250, 24, 0, 65, 78, 83, 73, 255, 240, // IAC SB TERMINAL_TYPE IS "ANSI" IAC SE
    ];

    let result = parser.parse(&terminal_response);
    if let Some(TelnetSequence::SubNegotiation { option, data }) = result.sequences.first() {
        println!("     Response sub-data: {:?}", data);
        println!(
            "     Interpretation: IS command (byte 0) + 'ANSI' ({:?})",
            String::from_utf8_lossy(&data[1..])
        );
        println!("     Meaning: 'My terminal type is ANSI'");
    }

    println!("\n5. Testing partial sequence parsing:");
    let mut parser2 = TelnetParser::new();

    // First chunk: IAC WILL (incomplete)
    println!("   Parsing incomplete sequence: [255, 251] (IAC WILL ?)");
    let result1 = parser2.parse(&[255, 251]);
    println!(
        "     Data: {} bytes, Commands: {}, Has buffered: {}",
        result1.data.len(),
        result1.sequences.len(),
        parser2.has_buffered_data()
    );

    // Second chunk: ECHO (completes the sequence)
    println!("   Adding final byte: [1] (ECHO option)");
    let result2 = parser2.parse(&[1]);
    println!(
        "     Data: {} bytes, Commands: {}, Complete sequence: {:?}",
        result2.data.len(),
        result2.sequences.len(),
        result2.sequences.first()
    );

    println!("\n6. Real-world telnet negotiation sequence:");
    let real_world = vec![
        255, 251, 1, // IAC WILL ECHO
        255, 251, 3, // IAC WILL SUPPRESS_GO_AHEAD
        255, 253, 31, // IAC DO NAWS (window size)
        255, 253, 24, // IAC DO TERMINAL_TYPE
    ];

    let result = parser.parse(&real_world);
    println!("   Typical BBS server negotiation:");
    println!("   Commands detected: {}", result.sequences.len());

    for (i, seq) in result.sequences.iter().enumerate() {
        if let TelnetSequence::Negotiation { command, option } = seq {
            let meaning = match (command, option) {
                (TelnetCommand::WILL, TelnetOption::ECHO) => "Server will handle echoing",
                (TelnetCommand::WILL, TelnetOption::SUPPRESS_GO_AHEAD) => {
                    "Server supports full-duplex"
                }
                (TelnetCommand::DO, TelnetOption::NAWS) => "Server wants window size info",
                (TelnetCommand::DO, TelnetOption::TERMINAL_TYPE) => "Server wants terminal type",
                _ => "Other negotiation",
            };
            println!("     {}: {:?} {:?} - {}", i + 1, command, option, meaning);
        }
    }

    println!("\n7. Parser state management:");
    let mut parser3 = TelnetParser::new();
    println!("   Initial state: {}", parser3.state());

    parser3.parse(&[255]); // IAC
    println!(
        "   After IAC: {} (buffered: {})",
        parser3.state(),
        parser3.has_buffered_data()
    );

    parser3.parse(&[251]); // WILL
    println!(
        "   After WILL: {} (buffered: {})",
        parser3.state(),
        parser3.has_buffered_data()
    );

    parser3.parse(&[1]); // ECHO
    println!(
        "   After ECHO: {} (buffered: {})",
        parser3.state(),
        parser3.has_buffered_data()
    );

    println!("\n=== Phase 3 Demo Complete ===");
    println!("This demonstrates:");
    println!("• IAC sequence detection from binary streams");
    println!("• Data/command separation");
    println!("• Stateful parsing across multiple chunks");
    println!("• Real-world telnet negotiation handling");
    println!("\nReady for Phase 4: Option Negotiation State Machine");
}
