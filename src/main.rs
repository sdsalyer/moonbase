use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

const TELNET_IP: &str = "127.0.0.1";
const TELNET_PORT: &str = "6969"; // 0 lets the OS decide
const TIMEOUT_SECS: u64 = 300;

fn main() -> std::io::Result<()> {
    let addr = format!("{TELNET_IP}:{TELNET_PORT}");
    let listener = TcpListener::bind(addr)?;

    // query the local address, i.e. if port assigned by OS
    let addr = listener.local_addr()?;

    println!("Moonbase BBS Server starting on {addr}");
    println!("> Connect with:  telnet {addr}");
    println!("> Press <Ctl+C> to stop the server");
    println!("> Listening...\n\n");

    // accept connections in a loop
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let peer_addr = stream
                    .peer_addr()
                    .unwrap_or_else(|_| "unknown".parse().unwrap());
                println!("> [{peer_addr}] connected.");

                thread::spawn(move || {
                    if let Err(e) = handle_client(stream) {
                        eprintln!("Error handling client {peer_addr}: {}", e);
                    }
                    println!("> [{peer_addr}] disconnected.");
                });
            }

            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }

    Ok(())
}

/// Handle client connections
fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    // set timeout to prevent hanging
    stream.set_read_timeout(Some(Duration::from_secs(TIMEOUT_SECS)))?;

    // say hello
    send_welcome(&mut stream)?;

    // wait for commands...
    let mut buffer = [0; 1024];
    loop {
        // Send a prompt
        stream.write_all(b"moonbase> ")?;
        stream.flush()?;

        // read user input
        match stream.read(&mut buffer) {
            Ok(0) => break, // Client disconnected

            Ok(n) => {
                let input = String::from_utf8_lossy(&buffer[0..n]);
                let command = input.trim();

                // Handle basic commands
                match command.to_lowercase().as_str() {
                    "help" => show_help(&mut stream)?,
                    "time" => show_time(&mut stream)?,
                    "echo" => echo_loop(&mut stream)?,
                    "quit" | "exit" | "bye" => {
                        stream.write_all(b"Goodbye!\r\n")?;
                        break;
                    }
                    "" => {
                        // empty command... just prompt again
                    }
                    _ => {
                        stream.write_all(
                            b"Unknown command. Type 'help' for available commands.\r\n",
                        )?;
                    }
                }
            }

            Err(e) => {
                eprintln!("Error reading from client: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn send_welcome(stream: &mut TcpStream) -> std::io::Result<()> {
    // TODO: read this from a baner.txt or other config
    let welcome = r#"
╔══════════════════════════════════════╗
║          Welcome to Moonbase         ║
║                                      ║
║     A nostalgic bulletin board       ║
║        system built in Rust          ║
╚══════════════════════════════════════╝

Type 'help' for available commands.

"#;
    stream.write_all(welcome.as_bytes())?;
    stream.flush()
}

fn show_help(stream: &mut TcpStream) -> std::io::Result<()> {
    let help_text = r#"
Available commands:
  help    - Show this help message
  time    - Display current server time
  echo    - Start echo test mode
  quit    - Disconnect from Moonbase (also: exit, bye)

"#;
    stream.write_all(help_text.as_bytes())?;
    stream.flush()
}

fn show_time(stream: &mut TcpStream) -> std::io::Result<()> {
    use jiff::Zoned;
    let local_time = Zoned::now();
    let time_str = format!("Moondate: {}\r\n", local_time.to_string());
    stream.write_all(time_str.as_bytes())?;
    stream.flush()
}

fn echo_loop(stream: &mut TcpStream) -> std::io::Result<()> {
    stream.write_all(b"Echo test - type something and press Enter ('done' to stop):\n\n")?;

    let mut buffer = [0; 1024];
    loop {
        stream.write_all(b">>> ")?;
        stream.flush()?;

        match stream.read(&mut buffer) {
            Ok(0) => break,

            Ok(n) => {
                let input = String::from_utf8_lossy(&buffer[0..n]);
                let text = input.trim();

                if text.to_lowercase() == "done" {
                    stream.write_all(b"Exiting echo mode.\r\n")?;
                    break;
                }

                let echo = format!("{}... {}... {}...\r\n", text, text, text);
                stream.write_all(echo.as_bytes())?;

            }

            Err(e) => {
                eprintln!("Error in echo loop: {}", e);
                break;
            }
        }
    }

    Ok(())
}
