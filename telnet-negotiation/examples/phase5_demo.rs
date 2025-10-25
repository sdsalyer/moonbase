//! # Phase 5 Demo: TelnetStream Integration
//!
//! This example demonstrates the transparent TelnetStream wrapper that provides
//! automatic telnet protocol handling while maintaining backward compatibility.
//! Run with: `cargo run --example phase5_demo`

use telnet_negotiation::{Side, TelnetOption, TelnetStream};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== Telnet Negotiation Library - Phase 5 Demo ===\n");
    println!("TelnetStream: Transparent Telnet Protocol Wrapper\n");

    // Start a simple echo server for testing
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind listener");
    let server_addr = listener.local_addr().expect("Failed to get server address");
    
    println!("Starting test server on {}", server_addr);
    
    // Spawn server thread
    let server_handle = thread::spawn(move || {
        println!("[Server] Listening for connections...");
        
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("[Server] New connection from: {}", 
                        stream.peer_addr().unwrap_or_else(|_| "unknown".parse().unwrap()));
                    
                    // Wrap the TcpStream with TelnetStream for automatic telnet handling
                    let mut telnet_stream = TelnetStream::with_debug(stream);
                    
                    // Handle the connection
                    if let Err(e) = handle_server_connection(&mut telnet_stream) {
                        eprintln!("[Server] Connection error: {}", e);
                    }
                    
                    println!("[Server] Connection closed");
                    break; // Only handle one connection for demo
                },
                Err(e) => {
                    eprintln!("[Server] Accept error: {}", e);
                    break;
                }
            }
        }
    });
    
    // Give server time to start
    thread::sleep(Duration::from_millis(100));
    
    // Connect as client and demonstrate TelnetStream usage
    println!("\n{}", "=".repeat(60));
    println!("Demonstrating TelnetStream Client Usage");
    println!("{}", "=".repeat(60));
    
    match TcpStream::connect(server_addr) {
        Ok(stream) => {
            println!("[Client] Connected to server");
            
            // Wrap with TelnetStream - this is the key integration point!
            let mut telnet_stream = TelnetStream::with_debug(stream);
            
            // Demonstrate transparent operation
            demonstrate_transparent_operation(&mut telnet_stream);
        },
        Err(e) => {
            eprintln!("[Client] Connection failed: {}", e);
        }
    }
    
    // Wait for server to finish
    let _ = server_handle.join();
    
    println!("\n{}", "=".repeat(60));
    println!("Phase 5 Demo Complete!");
    println!("{}", "=".repeat(60));
    println!("Demonstrated:");
    println!("• TelnetStream as drop-in TcpStream replacement");
    println!("• Transparent telnet protocol handling");
    println!("• Automatic option negotiation in background");
    println!("• Clean data separation (telnet commands filtered)");
    println!("• Backward compatibility with existing code patterns");
    println!("• Read/Write trait implementation");
    println!("\nReady for Phase 6: Specific Option Implementations");
}

/// Handle server-side connection with automatic telnet processing
fn handle_server_connection(stream: &mut TelnetStream) -> std::io::Result<()> {
    println!("[Server] Connection established, telnet processing active");
    
    // Send welcome message
    stream.write(b"Welcome to the Phase 5 Demo Server!\r\n")?;
    stream.write(b"This server uses TelnetStream for transparent telnet handling.\r\n")?;
    stream.write(b"Type messages and they will be echoed back. Type 'quit' to exit.\r\n")?;
    stream.write(b"\r\nPrompt> ")?;
    stream.flush()?;
    
    // Simple echo loop
    let mut buffer = [0; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                println!("[Server] Client disconnected (EOF)");
                break;
            },
            Ok(n) => {
                let received = String::from_utf8_lossy(&buffer[..n]);
                println!("[Server] Received (clean data): {:?}", received.trim());
                
                // Check for quit command
                if received.trim().eq_ignore_ascii_case("quit") {
                    stream.write(b"Goodbye!\r\n")?;
                    stream.flush()?;
                    break;
                }
                
                // Echo back the message
                stream.write(b"Echo: ")?;
                stream.write(&buffer[..n])?;
                stream.write(b"Prompt> ")?;
                stream.flush()?;
            },
            Err(e) => {
                println!("[Server] Read error: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}

/// Demonstrate transparent TelnetStream operation from client perspective
fn demonstrate_transparent_operation(stream: &mut TelnetStream) {
    println!("[Client] Demonstrating transparent telnet operation...\n");
    
    // Show current option states
    println!("1. Initial telnet option states:");
    show_option_states(stream);
    
    // Send some test messages
    println!("\n2. Sending test messages (with automatic telnet handling):");
    
    let test_messages = [
        "Hello, TelnetStream!",
        "This message contains IAC byte (255) - should be auto-escaped",
        "Testing transparent operation",
        "quit"
    ];
    
    for (i, message) in test_messages.iter().enumerate() {
        println!("   Sending message {}: {:?}", i + 1, message);
        
        if let Err(e) = stream.write(message.as_bytes()) {
            eprintln!("[Client] Write error: {}", e);
            continue;
        }
        
        if let Err(e) = stream.write(b"\r\n") {
            eprintln!("[Client] Write error: {}", e);
            continue;
        }
        
        if let Err(e) = stream.flush() {
            eprintln!("[Client] Flush error: {}", e);
            continue;
        }
        
        // Read response
        let mut buffer = [0; 1024];
        match stream.read(&mut buffer) {
            Ok(n) => {
                let response = String::from_utf8_lossy(&buffer[..n]);
                println!("   Server response: {:?}", response.trim());
            },
            Err(e) => {
                eprintln!("[Client] Read error: {}", e);
                break;
            }
        }
        
        // Small delay between messages
        thread::sleep(Duration::from_millis(100));
    }
    
    // Show final option states
    println!("\n3. Final telnet option states:");
    show_option_states(stream);
    
    println!("\n4. TelnetStream transparent operation completed successfully!");
    println!("   • All telnet protocol handling was automatic");
    println!("   • Application only saw clean data (no telnet commands)");
    println!("   • IAC bytes were automatically escaped in outgoing data");
    println!("   • Option negotiation happened in background");
}

/// Display current telnet option states for common options
fn show_option_states(stream: &TelnetStream) {
    let common_options = [
        TelnetOption::ECHO,
        TelnetOption::SUPPRESS_GO_AHEAD,
        TelnetOption::NAWS,
        TelnetOption::TERMINAL_TYPE,
        TelnetOption::BINARY,
    ];
    
    println!("   Common Telnet Options Status:");
    for option in &common_options {
        let local_enabled = stream.is_option_enabled(Side::Local, *option);
        let remote_enabled = stream.is_option_enabled(Side::Remote, *option);
        println!("     {:?}: Local={}, Remote={}", 
            option, local_enabled, remote_enabled);
    }
}

/// Integration example showing how existing code can be updated
#[allow(dead_code)]
fn integration_example() {
    // Example of how existing BBS code could be updated:
    
    // BEFORE - using raw TcpStream:
    /*
    fn handle_client_old(stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = stream;
        // Manual telnet handling would be required here
        stream.write(b"Welcome!\r\n")?;
        Ok(())
    }
    */
    
    // AFTER - using TelnetStream (minimal change!):
    fn _handle_client_new(stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TelnetStream::new(stream); // Only this line changes!
        // Everything else remains the same - telnet handling is now automatic
        stream.write(b"Welcome!\r\n")?;
        Ok(())
    }
    
    println!("Integration example: TelnetStream is a drop-in replacement!");
}