mod box_renderer;
mod bulletin_repository;
mod bulletins;
mod config;
mod errors;
mod menu;
mod message_repository;
mod messages;
mod services;
mod session;
mod user_repository;
mod users;

use box_renderer::BoxRenderer;
use bulletin_repository::JsonBulletinStorage;
use config::BbsConfig;
use errors::BbsResult;
use message_repository::JsonMessageStorage;
use services::CoreServices;
use session::BbsSession;
use user_repository::JsonUserStorage;

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;

/// Moonbase entry point
fn main() -> BbsResult<()> {
    // Load configuration
    let config = match BbsConfig::load_from_file("bbs.conf") {
        Ok(config) => {
            println!("Configuration loaded from bbs.conf");
            config
        }
        Err(e) => {
            eprintln!("Config error: {}. Using defaults.", e);
            BbsConfig::default()
        }
    };

    // Print startup information
    if let Err(e) = print_startup_banner(&config) {
        eprintln!("Runtime error: {}", e);
        return Err(e);
    }

    // Wrap config in Arc for sharing between threads
    let config = Arc::new(config);

    // Initialize shared user storage
    let user_storage = match JsonUserStorage::new("data") {
        Ok(storage) => {
            println!("+ User storage initialized");
            Arc::new(Mutex::new(storage))
        }
        Err(e) => {
            eprintln!("x Failed to initialize user storage: {}", e);
            return Err(e);
        }
    };

    // Initialize shared bulletin storage
    let bulletin_storage = match JsonBulletinStorage::new("data") {
        Ok(storage) => {
            println!("+ Bulletin storage initialized");
            Arc::new(Mutex::new(storage))
        }
        Err(e) => {
            eprintln!("x Failed to initialize bulletin storage: {}", e);
            return Err(e);
        }
    };

    // Initialize shared message storage
    let message_storage = match JsonMessageStorage::new("data") {
        Ok(storage) => {
            println!("+ Message storage initialized");
            Arc::new(Mutex::new(storage))
        }
        Err(e) => {
            eprintln!("x Failed to initialize message storage: {}", e);
            return Err(e);
        }
    };

    // Create services
    let services = Arc::new(CoreServices::new(
        user_storage.clone() as Arc<Mutex<dyn crate::user_repository::UserStorage + Send>>,
        bulletin_storage.clone()
            as Arc<Mutex<dyn crate::bulletin_repository::BulletinStorage + Send>>,
        message_storage.clone()
            as Arc<Mutex<dyn crate::message_repository::MessageStorage + Send>>,
    ));

    // Start the server
    let bind_addr = format!(
        "{}:{}",
        config.server.bind_address, config.server.telnet_port
    );
    let listener = TcpListener::bind(&bind_addr)?;

    println!("> {} starting on {}", config.bbs.name, bind_addr);
    println!(
        "> Connect with: telnet {} {}",
        config.server.bind_address, config.server.telnet_port
    );
    println!("> SysOp: {}", config.bbs.sysop_name);

    if config.features.allow_anonymous {
        println!("> Anonymous access: Enabled");
    } else {
        println!("> Anonymous access: Disabled");
    }

    println!("\nPress Ctrl+C to stop the server\n");

    // Accept connections with proper connection tracking
    let connection_count = Arc::new(AtomicU32::new(0));
    let mut connection_id = 0u32;
    
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                connection_id += 1;
                let current_connections = connection_count.fetch_add(1, Ordering::Relaxed) + 1;

                // Clone config for this thread
                let config = Arc::clone(&config);

                // Check connection limit
                if current_connections as usize > config.server.max_connections {
                    eprintln!("!  Connection limit reached ({}/{}), rejecting connection", 
                             current_connections, config.server.max_connections);
                    let _ = show_rejection(stream, config);
                    connection_count.fetch_sub(1, Ordering::Relaxed);
                    continue;
                }

                // TODO: Fix unwraps
                let peer_addr = stream
                    .peer_addr()
                    .unwrap_or_else(|_| "unknown".parse().unwrap());
                println!("> New connection #{} from: {} ({}/{})", 
                        connection_id, peer_addr, current_connections, config.server.max_connections);

                // Clone services and connection counter for this thread
                let services = Arc::clone(&services);
                let conn_counter = Arc::clone(&connection_count);

                // Spawn thread to handle connection
                thread::spawn(move || {
                    // Set connection timeout
                    if let Err(e) =
                        stream.set_read_timeout(Some(config.timeouts.connection_timeout))
                    {
                        eprintln!("Failed to set timeout for {}: {}", peer_addr, e);
                    }

                    // Handle the client session
                    match handle_client(stream, config, services) {
                        Ok(()) => {
                            let remaining = conn_counter.fetch_sub(1, Ordering::Relaxed) - 1;
                            println!("> Client {} disconnected normally ({} connections remaining)", 
                                   peer_addr, remaining);
                        },
                        Err(e) => {
                            let remaining = conn_counter.fetch_sub(1, Ordering::Relaxed) - 1;
                            eprintln!("! Error handling client {}: {} ({} connections remaining)", 
                                    peer_addr, e, remaining);
                        }
                    }
                });
            }

            Err(e) => {
                eprintln!("! Error accepting connection: {}", e);
            }
        }
    }

    Ok(())
}

/// Handle client BBS Session
fn handle_client(
    stream: TcpStream,
    config: Arc<BbsConfig>,
    services: Arc<CoreServices>,
) -> BbsResult<()> {
    let mut session = BbsSession::new(config, services);
    session.run(stream)
}

/// Show Server startup messages in console log
fn print_startup_banner(config: &BbsConfig) -> BbsResult<()> {
    let box_renderer = BoxRenderer::new(config.ui.box_style, config.ui.use_colors);

    let mut output = Vec::new();

    // Use owned Strings to avoid lifetime issues
    let banner_items: Vec<String> = vec![
        "*  RUST BBS SERVER  *".to_string(),
        "".to_string(),
        format!("BBS Name: {}", config.bbs.name),
        format!("Tagline:  {}", config.bbs.tagline),
        format!("SysOp:    {}", config.bbs.sysop_name),
        format!("Location: {}", config.bbs.location),
        "".to_string(),
        "Network Settings:".to_string(),
        format!("  Telnet Port: {}", config.server.telnet_port),
        config
            .server
            .ssh_port
            .map_or("  SSH Port:    Disabled".to_string(), |port| {
                format!("  SSH Port:    {}", port)
            }),
        format!("  Max Connections: {}", config.server.max_connections),
        format!(
            "  Connection Timeout: {}s",
            config.timeouts.connection_timeout.as_secs()
        ),
        "".to_string(),
        "UI Settings:".to_string(),
        format!("  Box Style: {:?}", config.ui.box_style),
        format!("  Menu Width: {}", config.ui.menu_width),
        format!(
            "  Colors: {}",
            if config.ui.use_colors {
                "Enabled"
            } else {
                "Disabled"
            }
        ),
    ];

    // Pass references to the owned strings
    box_renderer.render_box(&mut output, "SERVER CONFIGURATION", &banner_items, 70, None)?;

    print!("\n{}", String::from_utf8_lossy(&output));

    Ok(())
}

/// Notify user BBS connection limit has been reached
fn show_rejection(mut stream: TcpStream, config: Arc<BbsConfig>) -> BbsResult<()> {
    // Create a simple box renderer for the rejection message
    let box_renderer =
        crate::box_renderer::BoxRenderer::new(config.ui.box_style, config.ui.use_colors);

    let message = "Sorry, the BBS has reached its maximum number of concurrent connections. Please try again later.";

    box_renderer.render_message_box(
        &mut stream,
        "SERVER BUSY",
        message,
        config.ui.menu_width,
        Some(crossterm::style::Color::Red),
    )?;

    stream.write_all(b"\nConnection will close in 5 seconds...\n")?;
    stream.flush()?;

    // Brief pause before closing
    std::thread::sleep(std::time::Duration::from_secs(5));
    Ok(())
}
