//! # Telnet Test Client
//!
//! This tool connects to a telnet server and sends specific telnet commands
//! to test command detection functionality.
//! Run with: `cargo run --example telnet_test_client`

use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== Telnet Command Test Client ===");

    match TcpStream::connect("127.0.0.1:2323") {
        Ok(mut stream) => {
            println!("Connected to BBS server on 127.0.0.1:2323");

            // Send initial telnet negotiation
            println!("Sending telnet negotiation commands...");

            // IAC WILL ECHO (client will handle echoing)
            let will_echo = vec![255, 251, 1];
            stream.write_all(&will_echo).unwrap();
            println!("Sent: IAC WILL ECHO");

            // IAC WILL SUPPRESS_GO_AHEAD (client supports full-duplex)
            let will_sga = vec![255, 251, 3];
            stream.write_all(&will_sga).unwrap();
            println!("Sent: IAC WILL SUPPRESS_GO_AHEAD");

            // IAC DO NAWS (client wants window size negotiation)
            let do_naws = vec![255, 253, 31];
            stream.write_all(&do_naws).unwrap();
            println!("Sent: IAC DO NAWS");

            // IAC DO TERMINAL_TYPE (client wants terminal type)
            let do_ttype = vec![255, 253, 24];
            stream.write_all(&do_ttype).unwrap();
            println!("Sent: IAC DO TERMINAL_TYPE");

            stream.flush().unwrap();

            // Wait a moment for server processing
            thread::sleep(Duration::from_millis(100));

            // Send some regular data mixed with telnet commands
            println!("Sending mixed data and commands...");

            let mixed_data = vec![
                // "hello"
                104, 101, 108, 108, 111, // IAC WONT ECHO (stop echoing)
                255, 252, 1, // "\r\n"
                13, 10,
            ];
            stream.write_all(&mixed_data).unwrap();
            stream.flush().unwrap();

            println!("Sent mixed data with IAC WONT ECHO");

            // Wait for response and read server output
            thread::sleep(Duration::from_millis(500));

            let mut buffer = [0; 4096];
            match stream.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    println!("Received {} bytes from server:", n);
                    let response = String::from_utf8_lossy(&buffer[0..n]);
                    println!("Response: {:?}", response);
                }
                Ok(_) => {
                    println!("Server closed connection");
                }
                Err(e) => {
                    println!("Error reading from server: {}", e);
                }
            }

            // Send quit command
            stream.write_all(b"quit\r\n").unwrap();
            stream.flush().unwrap();
            println!("Sent quit command");

            thread::sleep(Duration::from_millis(100));
        }
        Err(e) => {
            println!("Failed to connect to server: {}", e);
            println!("Make sure the BBS server is running on 127.0.0.1:2323");
            println!("Start it with: cargo run");
        }
    }

    println!("Test client finished");
}
