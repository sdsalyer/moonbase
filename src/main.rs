mod box_renderer;
mod config;
mod errors;
mod menu;
mod session;

use config::BbsConfig;
use session::BbsSession;
use errors::{BbsResult, BbsError};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

use crate::box_renderer::{BoxRenderer, BoxStyle};

fn main() -> BbsResult<()> {
    // Load configuration
    let config = match BbsConfig::load_from_file("bbs.conf") {
        Ok(config) => {
            println!("✓ Configuration loaded from bbs.conf");
            config
        },
        Err(e) => {
            eprintln!("Config error: {}. Using defaults.", e);
            BbsConfig::default()
        }
    };
    
    // Print startup information
    print_startup_banner(&config);
    
    // Wrap config in Arc for sharing between threads
    let config = Arc::new(config);
    
    // Start the server
    let bind_addr = format!("{}:{}", config.server.bind_address, config.server.telnet_port);
    let listener = TcpListener::bind(&bind_addr)?;
    
    println!("🚀 {} starting on {}", config.bbs.name, bind_addr);
    println!("📞 Connect with: telnet {} {}", config.server.bind_address, config.server.telnet_port);
    println!("👤 SysOp: {}", config.bbs.sysop_name);
    
    if config.features.allow_anonymous {
        println!("🌐 Anonymous access: Enabled");
    } else {
        println!("🔒 Anonymous access: Disabled");
    }
    
    println!("\nPress Ctrl+C to stop the server\n");
    
    // Accept connections
    let mut connection_count = 0;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                connection_count += 1;
                
                // Check connection limit
                if connection_count > config.server.max_connections {
                    eprintln!("⚠️  Connection limit reached, rejecting connection");
                    let _ = send_rejection_message(stream);
                    connection_count -= 1;
                    continue;
                }
                
                let peer_addr = stream.peer_addr()
                    .unwrap_or_else(|_| "unknown".parse().unwrap());
                println!("📞 New connection #{} from: {}", connection_count, peer_addr);
                
                // Clone config for this thread
                let config = Arc::clone(&config);
                
                // Spawn thread to handle connection
                thread::spawn(move || {
                    // Set connection timeout
                    if let Err(e) = stream.set_read_timeout(Some(config.timeouts.connection_timeout)) {
                        eprintln!("Failed to set timeout for {}: {}", peer_addr, e);
                    }
                    
                    // Handle the client session
                    match handle_client(stream, config) {
                        Ok(()) => println!("✓ Client {} disconnected normally", peer_addr),
                        Err(e) => eprintln!("❌ Error handling client {}: {}", peer_addr, e),
                    }
                });
            }
            Err(e) => {
                eprintln!("❌ Error accepting connection: {}", e);
            }
        }
    }
    
    Ok(())
}

pub fn print_startup_banner(config: &BbsConfig) -> std::io::Result<()> {
    let box_renderer = BoxRenderer::new(BoxStyle::ascii());
    
    let mut output = Vec::new();
    
    // Use owned Strings to avoid lifetime issues
    let banner_items: Vec<String> = vec![
        "🏛️  RUST BBS SERVER  🏛️".to_string(),
        "".to_string(),
        format!("BBS Name: {}", config.bbs.name),
        format!("Tagline:  {}", config.bbs.tagline),
        format!("SysOp:    {}", config.bbs.sysop_name),
        format!("Location: {}", config.bbs.location),
        "".to_string(),
        "Network Settings:".to_string(),
        format!("  Telnet Port: {}", config.server.telnet_port),
        config.server.ssh_port
            .map_or("  SSH Port:    Disabled".to_string(), |port| 
                format!("  SSH Port:    {}", port)),
        format!("  Max Connections: {}", config.server.max_connections),
        format!("  Connection Timeout: {}s", config.timeouts.connection_timeout.as_secs()),
        "".to_string(),
        "UI Settings:".to_string(),
        format!("  Box Style: {:?}", config.ui.box_style),
        format!("  Menu Width: {}", config.ui.menu_width),
        format!("  Colors: {}", if config.ui.use_colors { "Enabled" } else { "Disabled" }),
    ];
    
    // Pass references to the owned strings
    box_renderer.render_box(&mut output, "SERVER CONFIGURATION", &banner_items, 70, None)?;
    
    print!("\n{}", String::from_utf8_lossy(&output));
    
    Ok(())
}

fn handle_client(stream: TcpStream, config: Arc<BbsConfig>) -> BbsResult<()> {
    let mut session = BbsSession::new(config);
    session.run(stream)
}

fn send_rejection_message(mut stream: TcpStream) -> std::io::Result<()> {
    // Create a simple box renderer for the rejection message
    let box_renderer = crate::box_renderer::BoxRenderer::new(crate::box_renderer::BoxStyle::double());
    
    let message = "Sorry, the BBS has reached its maximum number of concurrent connections. Please try again later.";
    
    box_renderer.render_message_box(&mut stream, "SERVER BUSY", message, 60, Some(crossterm::style::Color::Red))?;
    
    stream.write_all(b"\nConnection will close in 5 seconds...\n")?;
    stream.flush()?;
    
    // Brief pause before closing
    std::thread::sleep(std::time::Duration::from_secs(5));
    Ok(())
}
