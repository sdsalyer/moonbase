mod config;
// mod session;

use config::MoonbaseConfig;
// use session::MoonbaseSession;

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread,
    time::Duration,
};

const TIMEOUT_SECS: u64 = 300;

fn main() -> std::io::Result<()> {
    let config = match MoonbaseConfig::load_from_file("moonbase.conf") {
        Ok(config) => {
            println!("Configuration loaded from moonbase.conf");
            config
        }

        Err(e) => {
            eprintln!("Unable to load configuration from file: {}", e);
            eprintln!("Loading default configuration.");
            MoonbaseConfig::default()
        }
    };

    // store config on the heap for sharing between threads
    let config = Arc::new(config);

    let addr = format!(
        "{}:{}",
        config.server.bind_address, config.server.telnet_port
    );
    let listener = TcpListener::bind(addr)?;

    // query the local address, i.e. if port assigned by OS
    let addr = listener.local_addr()?;

    println!("{} BBS Server starting on {}", config.bbs.name, addr);
    println!("> SysOp: {}", config.bbs.sysop);
    println!("> Connect with:  telnet {}", addr);
    println!("> Press <Ctl+C> to stop the server");
    println!("> Listening...\n\n");

    // accept connections in a loop, up to the max
    let mut connection_count = 0;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                connection_count += 1;

                // Don't go over the configured limit
                if connection_count > config.server.max_connections {
                    eprintln!("Connection rejected: too many active connections");
                    let _ = send_rejection_message(stream);
                    connection_count -= 1;
                    continue;
                }

                let peer_addr = stream
                    .peer_addr()
                    .unwrap_or_else(|_| "unknown".parse().unwrap());
                println!("> [{peer_addr}] connected.");

                // Clone config reference for this thread
                let config = Arc::clone(&config);

                // Captured variables (stream, peer_addr) are moved into the
                // closure -- borrowing wouldn't work because main() might
                // not outlive the thread.
                thread::spawn(move || {
                    if let Err(e) = handle_client(stream, config) {
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
fn handle_client(mut stream: TcpStream, config: Arc<MoonbaseConfig>) -> std::io::Result<()> {
    // set timeout to prevent hanging
    stream.set_read_timeout(Some(config.server.connection_timeout))?;

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

fn send_rejection_message(mut stream: TcpStream) -> std::io::Result<()> {
    let message = r#"
╔══════════════════════════════════════╗
║          Welcome to Moonbase         ║
║                                      ║
║Sorry, the BBS has reached its maximum║
║number of concurrent connections.     ║
║                                      ║
║Please try again later.               ║
║                                      ║
║Connection will close in 5 seconds... ║
╚══════════════════════════════════════╝
 
"#;
    stream.write_all(message.as_bytes())?;
    stream.flush()?;

    // Brief pause before closing
    std::thread::sleep(std::time::Duration::from_secs(5));
    Ok(())
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
