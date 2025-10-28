# BBS Doors Implementation Plan - PTY Approach

## 🎯 Overview

Implement classic BBS "Doors" functionality that allows launching external programs (games, utilities) while maintaining the telnet session. External programs run in pseudo-terminals (PTYs) for full terminal compatibility, with safe I/O bridging between the door program and the telnet client.

**Key Benefits of PTY Approach:**
- ✅ **Zero unsafe code** - `portable-pty` crate handles all unsafe operations
- ✅ **Full terminal emulation** - Doors get real terminal environment with size, colors, control sequences
- ✅ **Cross-platform** - Works on Unix, Linux, macOS, and Windows
- ✅ **Professional grade** - Same approach used by terminal emulators and SSH servers

## 🏗 Implementation Phases

### Phase 1: Foundation & Configuration

#### 1.1 Dependencies
Add to `Cargo.toml`:
```toml
[dependencies]
portable-pty = "0.9"
# No additional dependencies needed - using atomic counter for session IDs
```

#### 1.2 Configuration Enhancement
**File**: `src/config.rs`

Add new configuration structures:
```rust
#[derive(Debug, Clone)]
pub struct DoorConfig {
    pub enabled: bool,
    pub directory: String,
    pub timeout_seconds: u64,
    pub max_execution_minutes: u64,
    pub allow_anonymous: bool,
    pub max_concurrent: usize,
}

// Add to FeatureConfig
pub struct FeatureConfig {
    // ... existing fields
    pub doors: DoorConfig,
}
```

Default configuration:
```rust
doors: DoorConfig {
    enabled: true,
    directory: "./doors".to_string(),
    timeout_seconds: 300,
    max_execution_minutes: 60, 
    allow_anonymous: false,
    max_concurrent: 3,
}
```

#### 1.3 Sample bbs.conf additions:
```toml
[features]
doors_enabled = true
doors_allow_anonymous = false

[doors]
directory = "./doors"
timeout_seconds = 300
max_execution_minutes = 60
max_concurrent = 3
```

### Phase 2: Core Data Structures

#### 2.1 Door Definition Structure
**New File**: `src/doors.rs`

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Door {
    pub id: String,
    pub name: String,
    pub description: String,
    pub command: String,
    pub arguments: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub environment: std::collections::HashMap<String, String>,
    
    // Access control
    pub requires_login: bool,
    pub allowed_users: Option<Vec<String>>,  // None = all users
    pub min_user_level: Option<u32>,
    
    // Resource limits
    pub max_time_minutes: Option<u64>,
    pub memory_limit_mb: Option<u64>,
    
    // Metadata
    pub category: DoorCategory,
    pub author: Option<String>,
    pub version: Option<String>,
    pub help_text: Option<String>,
    
    // Statistics
    pub times_run: u64,
    pub last_run: Option<chrono::DateTime<chrono::Utc>>,
    pub average_runtime_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DoorCategory {
    Game,
    Utility, 
    Communication,
    Information,
    Education,
    Entertainment,
    System,
    Custom(String),
}

#[derive(Debug)]
pub enum DoorError {
    NotFound(String),
    AccessDenied(String),
    AlreadyRunning,
    ExecutionFailed(String),
    Timeout,
    ResourceLimit(String),
}

pub type DoorResult<T> = Result<T, DoorError>;
```

#### 2.2 Door Repository
**New File**: `src/door_repository.rs`

```rust
use crate::doors::*;
use crate::errors::BbsResult;
use std::collections::HashMap;

pub trait DoorStorage: Send + Sync {
    fn list_doors(&self) -> BbsResult<Vec<Door>>;
    fn get_door(&self, id: &str) -> BbsResult<Option<Door>>;
    fn add_door(&mut self, door: Door) -> BbsResult<()>;
    fn update_door(&mut self, door: Door) -> BbsResult<()>;
    fn remove_door(&mut self, id: &str) -> BbsResult<bool>;
    fn get_doors_by_category(&self, category: &DoorCategory) -> BbsResult<Vec<Door>>;
    fn increment_run_count(&mut self, id: &str) -> BbsResult<()>;
    fn update_runtime_stats(&mut self, id: &str, runtime_seconds: f64) -> BbsResult<()>;
}

pub struct JsonDoorStorage {
    data_dir: PathBuf,
    doors_file: PathBuf,
    doors_cache: HashMap<String, Door>,
    last_loaded: Option<std::time::SystemTime>,
}

impl JsonDoorStorage {
    pub fn new(data_dir: &str) -> BbsResult<Self>;
    pub fn load_doors(&mut self) -> BbsResult<()>;
    pub fn save_doors(&self) -> BbsResult<()>;
    pub fn reload_if_changed(&mut self) -> BbsResult<()>;
}

impl DoorStorage for JsonDoorStorage {
    // Implementation of trait methods
}
```

### Phase 3: PTY Door Execution Engine

#### 3.1 Door Runner Core
**New File**: `src/door_runner.rs`

```rust
use portable_pty::{native_pty_system, CommandBuilder, PtySize, PtyPair, Child};
use telnet_negotiation::TelnetStream;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

// Global atomic counter for unique session IDs
static DOOR_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

pub struct DoorRunner {
    door: Door,
    session_id: u64,
    pty_pair: Option<PtyPair>,
    child: Option<Box<dyn Child + Send + Sync>>,
    start_time: Option<Instant>,
    status: DoorStatus,
    
    // I/O bridging threads
    input_thread: Option<thread::JoinHandle<()>>,
    output_thread: Option<thread::JoinHandle<()>>,
    
    // Termination signaling
    should_terminate: Arc<Mutex<bool>>,
}

#[derive(Debug, Clone)]
pub enum DoorStatus {
    NotStarted,
    Starting,
    Running,
    Completed(i32),
    Terminated,
    TimedOut,
    Error(String),
}

impl DoorRunner {
    pub fn new(door: Door) -> Self {
        Self {
            door,
            session_id: DOOR_SESSION_COUNTER.fetch_add(1, Ordering::SeqCst),
            pty_pair: None,
            child: None,
            start_time: None,
            status: DoorStatus::NotStarted,
            input_thread: None,
            output_thread: None,
            should_terminate: Arc::new(Mutex::new(false)),
        }
    }
    
    pub fn launch(
        &mut self,
        stream: &mut TelnetStream,
        session: &BbsSession,
    ) -> DoorResult<()> {
        // 1. Validate permissions
        self.validate_access(session)?;
        
        // 2. Setup PTY with proper terminal size
        self.setup_pty(session)?;
        
        // 3. Prepare environment variables
        self.setup_environment(session);
        
        // 4. Launch the door process
        self.spawn_process()?;
        
        // 5. Start I/O bridging threads
        self.start_io_bridging(stream)?;
        
        // 6. Monitor execution
        self.monitor_execution()?;
        
        Ok(())
    }
    
    fn validate_access(&self, session: &BbsSession) -> DoorResult<()> {
        // Check if user is logged in (if required)
        if self.door.requires_login && session.user.is_none() {
            return Err(DoorError::AccessDenied("Login required".to_string()));
        }
        
        // Check user-specific permissions
        if let Some(allowed_users) = &self.door.allowed_users {
            if let Some(user) = &session.user {
                if !allowed_users.contains(&user.username) {
                    return Err(DoorError::AccessDenied("User not permitted".to_string()));
                }
            }
        }
        
        Ok(())
    }
    
    fn setup_pty(&mut self, session: &BbsSession) -> DoorResult<()> {
        let pty_system = native_pty_system();
        
        // Use detected terminal size from Phase 7 capabilities
        let size = PtySize {
            rows: session.terminal_capabilities.height.unwrap_or(24) as u16,
            cols: session.effective_width() as u16,
            pixel_width: 0,
            pixel_height: 0,
        };
        
        self.pty_pair = Some(pty_system.openpty(size)
            .map_err(|e| DoorError::ExecutionFailed(format!("PTY creation failed: {}", e)))?);
        
        Ok(())
    }
    
    fn setup_environment(&self, session: &BbsSession) -> std::collections::HashMap<String, String> {
        let mut env = std::env::vars().collect::<std::collections::HashMap<_, _>>();
        
        // BBS-specific environment variables
        env.insert("BBS_NAME".to_string(), session.config.bbs.name.clone());
        env.insert("BBS_VERSION".to_string(), "1.0.0".to_string());
        env.insert("BBS_SYSOP".to_string(), session.config.bbs.sysop_name.clone());
        env.insert("BBS_NODE".to_string(), "1".to_string());
        env.insert("BBS_SESSION_ID".to_string(), self.session_id.to_string());
        
        // User information
        if let Some(user) = &session.user {
            env.insert("BBS_USER".to_string(), user.username.clone());
            env.insert("BBS_REALNAME".to_string(), user.real_name.clone());
            env.insert("BBS_LOCATION".to_string(), user.location.clone());
        } else {
            env.insert("BBS_USER".to_string(), "GUEST".to_string());
        }
        
        // Terminal capabilities from Phase 7
        env.insert("BBS_TERM_WIDTH".to_string(), session.effective_width().to_string());
        env.insert("BBS_TERM_HEIGHT".to_string(), 
                   session.terminal_capabilities.height.unwrap_or(24).to_string());
        env.insert("BBS_TERM_ANSI".to_string(), 
                   session.terminal_capabilities.supports_ansi.to_string());
        env.insert("BBS_TERM_COLOR".to_string(), 
                   session.terminal_capabilities.supports_color.to_string());
        
        // Door-specific environment variables
        for (key, value) in &self.door.environment {
            env.insert(key.clone(), value.clone());
        }
        
        env
    }
    
    fn spawn_process(&mut self) -> DoorResult<()> {
        let pty_pair = self.pty_pair.as_ref().unwrap();
        
        let mut cmd = CommandBuilder::new(&self.door.command);
        
        // Add arguments
        for arg in &self.door.arguments {
            cmd.arg(arg);
        }
        
        // Set working directory
        if let Some(work_dir) = &self.door.working_directory {
            cmd.cwd(work_dir);
        }
        
        // Set environment variables
        let env = self.setup_environment(session);
        for (key, value) in env {
            cmd.env(key, value);
        }
        
        // Spawn in PTY slave
        self.child = Some(pty_pair.slave.spawn_command(cmd)
            .map_err(|e| DoorError::ExecutionFailed(format!("Process spawn failed: {}", e)))?);
        
        self.status = DoorStatus::Running;
        self.start_time = Some(Instant::now());
        
        Ok(())
    }
    
    fn start_io_bridging(&mut self, stream: &mut TelnetStream) -> DoorResult<()> {
        let pty_master = Arc::new(Mutex::new(
            self.pty_pair.as_ref().unwrap().master.clone()
        ));
        
        let should_terminate = self.should_terminate.clone();
        
        // Thread 1: TelnetStream -> PTY (user input to door)
        let input_stream = /* clone/reference to telnet stream */;
        let input_pty = pty_master.clone();
        let input_terminate = should_terminate.clone();
        
        self.input_thread = Some(thread::spawn(move || {
            Self::bridge_input(input_stream, input_pty, input_terminate);
        }));
        
        // Thread 2: PTY -> TelnetStream (door output to user)  
        let output_stream = /* clone/reference to telnet stream */;
        let output_pty = pty_master.clone();
        let output_terminate = should_terminate.clone();
        
        self.output_thread = Some(thread::spawn(move || {
            Self::bridge_output(output_pty, output_stream, output_terminate);
        }));
        
        Ok(())
    }
    
    fn bridge_input(
        mut stream: /* TelnetStream ref */,
        pty: Arc<Mutex</* PTY Master */>>,
        should_terminate: Arc<Mutex<bool>>
    ) {
        let mut buffer = [0u8; 1024];
        
        loop {
            if *should_terminate.lock().unwrap() {
                break;
            }
            
            match stream.read(&mut buffer) {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    if let Ok(mut pty_master) = pty.lock() {
                        if pty_master.write_all(&buffer[0..n]).is_err() {
                            break; // PTY closed
                        }
                    }
                }
                Err(_) => break,
            }
        }
    }
    
    fn bridge_output(
        pty: Arc<Mutex</* PTY Master */>>,
        mut stream: /* TelnetStream ref */,
        should_terminate: Arc<Mutex<bool>>
    ) {
        let mut buffer = [0u8; 1024];
        
        loop {
            if *should_terminate.lock().unwrap() {
                break;
            }
            
            if let Ok(mut pty_master) = pty.lock() {
                match pty_master.read(&mut buffer) {
                    Ok(0) => break, // PTY closed
                    Ok(n) => {
                        if stream.write_all(&buffer[0..n]).is_err() {
                            break; // Connection closed
                        }
                        let _ = stream.flush();
                    }
                    Err(_) => break,
                }
            }
        }
    }
    
    fn monitor_execution(&mut self) -> DoorResult<()> {
        let max_duration = Duration::from_secs(
            self.door.max_time_minutes.unwrap_or(60) * 60
        );
        
        loop {
            // Check if child process is still running
            if let Some(ref mut child) = self.child {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        self.status = DoorStatus::Completed(status.success() as i32);
                        self.cleanup();
                        return Ok(());
                    }
                    Ok(None) => {
                        // Still running - check timeout
                        if let Some(start) = self.start_time {
                            if start.elapsed() > max_duration {
                                self.terminate()?;
                                self.status = DoorStatus::TimedOut;
                                return Err(DoorError::Timeout);
                            }
                        }
                    }
                    Err(e) => {
                        self.status = DoorStatus::Error(format!("Process error: {}", e));
                        self.cleanup();
                        return Err(DoorError::ExecutionFailed(e.to_string()));
                    }
                }
            }
            
            // Brief sleep to avoid busy waiting
            thread::sleep(Duration::from_millis(100));
        }
    }
    
    pub fn terminate(&mut self) -> DoorResult<()> {
        // Signal I/O threads to stop
        *self.should_terminate.lock().unwrap() = true;
        
        // Terminate child process
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
        }
        
        self.cleanup();
        self.status = DoorStatus::Terminated;
        
        Ok(())
    }
    
    fn cleanup(&mut self) {
        // Wait for I/O threads to finish
        if let Some(handle) = self.input_thread.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.output_thread.take() {
            let _ = handle.join();
        }
        
        // Clean up PTY resources
        self.pty_pair = None;
        self.child = None;
    }
    
    pub fn get_status(&self) -> &DoorStatus {
        &self.status
    }
    
    pub fn get_runtime(&self) -> Option<Duration> {
        self.start_time.map(|start| start.elapsed())
    }
}

impl Drop for DoorRunner {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}
```

### Phase 4: Menu System Integration

#### 4.1 Menu Actions Extension
**File**: `src/menu/mod.rs`

Add new menu actions:
```rust
pub enum MenuAction {
    // ... existing actions
    
    // Door-related actions
    DoorList,
    DoorCategory(DoorCategory),
    DoorLaunch(String),        // door_id
    DoorInfo(String),          // door_id
    DoorTerminate,             // Emergency exit from door
}
```

#### 4.2 Door Menu Implementation  
**New File**: `src/menu/menu_door.rs`

```rust
use super::{Menu, MenuAction, MenuRender, MenuScreen};
use crate::doors::*;
use crate::session::BbsSession;
use crate::box_renderer::MenuItem;

pub struct DoorMenu {
    state: DoorMenuState,
    selected_category: Option<DoorCategory>,
    available_doors: Vec<Door>,
    door_runner: Option<DoorRunner>,
}

#[derive(Debug, Clone)]
pub enum DoorMenuState {
    CategoryList,
    DoorList(DoorCategory),  
    DoorInfo(String),
    DoorRunning(String),
    DoorCompleted(String),
}

impl DoorMenu {
    pub fn new() -> Self {
        Self {
            state: DoorMenuState::CategoryList,
            selected_category: None,
            available_doors: Vec::new(),
            door_runner: None,
        }
    }
    
    fn load_available_doors(&mut self, session: &BbsSession) -> crate::errors::BbsResult<()> {
        // Load doors from repository, filtered by user permissions
        Ok(())
    }
}

impl MenuScreen for DoorMenu {
    fn render(&self, session: &BbsSession) -> MenuRender {
        match &self.state {
            DoorMenuState::CategoryList => self.render_categories(session),
            DoorMenuState::DoorList(category) => self.render_door_list(session, category),
            DoorMenuState::DoorInfo(door_id) => self.render_door_info(session, door_id),
            DoorMenuState::DoorRunning(door_id) => self.render_running_status(session, door_id),
            DoorMenuState::DoorCompleted(door_id) => self.render_completion(session, door_id),
        }
    }
    
    fn handle_input(&mut self, session: &BbsSession, input: &str) -> MenuAction {
        match &self.state {
            DoorMenuState::CategoryList => self.handle_category_input(input),
            DoorMenuState::DoorList(_) => self.handle_door_list_input(input),
            DoorMenuState::DoorInfo(_) => self.handle_info_input(input),
            DoorMenuState::DoorRunning(_) => self.handle_running_input(input),
            DoorMenuState::DoorCompleted(_) => self.handle_completion_input(input),
        }
    }
}

impl DoorMenu {
    fn render_categories(&self, session: &BbsSession) -> MenuRender {
        let title = "DOOR CATEGORIES";
        let mut items = vec![
            MenuItem::info("Select a category to browse available doors:"),
            MenuItem::separator(),
        ];
        
        // Add category options
        items.extend([
            MenuItem::option("G", "Games"),
            MenuItem::option("U", "Utilities"), 
            MenuItem::option("I", "Information"),
            MenuItem::option("E", "Entertainment"),
            MenuItem::separator(),
            MenuItem::option("A", "All Doors"),
            MenuItem::separator(),
            MenuItem::option("Q", "Return to Main Menu"),
        ]);
        
        MenuRender {
            title: title.to_string(),
            items,
        }
    }
    
    fn render_door_list(&self, session: &BbsSession, category: &DoorCategory) -> MenuRender {
        // Implementation for door listing
        todo!()
    }
    
    fn render_door_info(&self, session: &BbsSession, door_id: &str) -> MenuRender {
        // Implementation for door information display
        todo!()
    }
    
    // ... other render methods
}
```

### Phase 5: Session Integration

#### 5.1 Session Door Management
**File**: `src/session.rs`

Add door-related functionality to BbsSession:

```rust
impl BbsSession {
    fn menu_handle_action(
        &mut self,
        stream: &mut TelnetStream,
        action: MenuAction,
    ) -> BbsResult<bool> {
        match action {
            // ... existing action handlers
            
            MenuAction::DoorList => {
                self.menu_current = Menu::Door;
                Ok(true)
            }
            
            MenuAction::DoorLaunch(door_id) => {
                self.handle_door_launch(stream, &door_id)
            }
            
            MenuAction::DoorInfo(door_id) => {
                self.handle_door_info(stream, &door_id)
            }
            
            MenuAction::DoorTerminate => {
                self.handle_door_terminate(stream)
            }
            
            // ... other handlers
        }
    }
    
    fn handle_door_launch(
        &mut self, 
        stream: &mut TelnetStream, 
        door_id: &str
    ) -> BbsResult<bool> {
        // 1. Validate door exists and user has permission
        let door = self.services.doors.get_door(door_id)?
            .ok_or_else(|| BbsError::InvalidInput(format!("Door '{}' not found", door_id)))?;
        
        // 2. Display pre-door message
        self.show_message_with_stream(
            stream,
            "LAUNCHING DOOR",
            &format!("Starting {}...\n\nPress Ctrl+] to return to BBS if the door becomes unresponsive.", door.name),
            Some(Color::Yellow),
        )?;
        
        // 3. Create and launch door runner
        let mut runner = DoorRunner::new(door.clone());
        
        match runner.launch(stream, self) {
            Ok(()) => {
                // 4. Update door statistics
                self.services.doors.increment_run_count(door_id)?;
                
                // 5. Display completion message
                self.show_message_with_stream(
                    stream,
                    "DOOR COMPLETED", 
                    &format!("{} has finished executing.\nReturning to BBS...", door.name),
                    Some(Color::Green),
                )?;
            }
            
            Err(DoorError::AccessDenied(msg)) => {
                self.show_message_with_stream(
                    stream,
                    "ACCESS DENIED",
                    &format!("Cannot launch door: {}", msg),
                    Some(Color::Red),
                )?;
            }
            
            Err(DoorError::Timeout) => {
                self.show_message_with_stream(
                    stream,
                    "DOOR TIMEOUT",
                    &format!("Door '{}' exceeded maximum execution time and was terminated.", door.name),
                    Some(Color::Yellow),
                )?;
            }
            
            Err(e) => {
                self.show_message_with_stream(
                    stream,
                    "DOOR ERROR",
                    &format!("Error executing door: {:?}", e),
                    Some(Color::Red),
                )?;
            }
        }
        
        Ok(true)
    }
    
    fn handle_door_info(&mut self, stream: &mut TelnetStream, door_id: &str) -> BbsResult<bool> {
        let door = self.services.doors.get_door(door_id)?
            .ok_or_else(|| BbsError::InvalidInput(format!("Door '{}' not found", door_id)))?;
        
        let info_text = format!(
            "Name: {}\nDescription: {}\nAuthor: {}\nCategory: {:?}\nRuns: {}\n\n{}",
            door.name,
            door.description,
            door.author.as_deref().unwrap_or("Unknown"),
            door.category,
            door.times_run,
            door.help_text.as_deref().unwrap_or("No additional information available.")
        );
        
        self.show_message_with_stream(
            stream,
            &format!("DOOR INFO: {}", door.name.to_uppercase()),
            &info_text,
            Some(Color::Cyan),
        )?;
        
        Ok(true)
    }
    
    fn handle_door_terminate(&mut self, stream: &mut TelnetStream) -> BbsResult<bool> {
        // Emergency termination - if somehow a door is still running
        self.show_message_with_stream(
            stream,
            "EMERGENCY EXIT",
            "Returned to BBS. If you were running a door, it may have been terminated.",
            Some(Color::Yellow),
        )?;
        
        self.menu_current = Menu::Main;
        Ok(true)
    }
}
```

### Phase 6: Service Layer Integration

#### 6.1 Door Service
**New File**: `src/services/door_service.rs`

```rust
use crate::doors::*;
use crate::door_repository::*;
use crate::errors::BbsResult;
use std::sync::{Arc, Mutex};

pub struct DoorService {
    storage: Arc<Mutex<dyn DoorStorage>>,
}

impl DoorService {
    pub fn new(storage: Arc<Mutex<dyn DoorStorage>>) -> Self {
        Self { storage }
    }
    
    pub fn list_doors(&self) -> BbsResult<Vec<Door>> {
        self.storage.lock().unwrap().list_doors()
    }
    
    pub fn get_door(&self, id: &str) -> BbsResult<Option<Door>> {
        self.storage.lock().unwrap().get_door(id)
    }
    
    pub fn get_doors_by_category(&self, category: &DoorCategory) -> BbsResult<Vec<Door>> {
        self.storage.lock().unwrap().get_doors_by_category(category)
    }
    
    pub fn increment_run_count(&self, id: &str) -> BbsResult<()> {
        self.storage.lock().unwrap().increment_run_count(id)
    }
    
    pub fn record_execution(&self, id: &str, duration: std::time::Duration) -> BbsResult<()> {
        self.storage.lock().unwrap().update_runtime_stats(id, duration.as_secs_f64())
    }
    
    pub fn reload_doors(&self) -> BbsResult<()> {
        if let Ok(mut storage) = self.storage.lock() {
            if let Some(json_storage) = storage.as_any_mut().downcast_mut::<JsonDoorStorage>() {
                json_storage.reload_if_changed()?;
            }
        }
        Ok(())
    }
}
```

#### 6.2 Core Services Integration
**File**: `src/services/mod.rs`

```rust
pub struct CoreServices {
    pub users: UserService,
    pub bulletins: BulletinService,
    pub messages: MessageService,
    pub doors: DoorService,  // Add this
}

impl CoreServices {
    pub fn new(
        user_storage: Arc<Mutex<dyn UserStorage + Send>>,
        bulletin_storage: Arc<Mutex<dyn BulletinStorage + Send>>,
        message_storage: Arc<Mutex<dyn MessageStorage + Send>>,
        door_storage: Arc<Mutex<dyn DoorStorage + Send>>,  // Add this parameter
    ) -> Self {
        Self {
            users: UserService::new(user_storage),
            bulletins: BulletinService::new(bulletin_storage),
            messages: MessageService::new(message_storage),
            doors: DoorService::new(door_storage),  // Add this
        }
    }
}
```

### Phase 7: Sample Doors & Testing

#### 7.1 Sample Door Definitions
**File**: `doors/doors.json`

```json
{
  "doors": [
    {
      "id": "welcome",
      "name": "Welcome Message",
      "description": "Display a welcome message with system information",
      "command": "/bin/echo",
      "arguments": ["Welcome to the BBS! Today is $(date)"],
      "working_directory": null,
      "environment": {},
      "requires_login": false,
      "allowed_users": null,
      "min_user_level": null,
      "max_time_minutes": 1,
      "memory_limit_mb": null,
      "category": "Utility",
      "author": "BBS System",
      "version": "1.0",
      "help_text": "Simple welcome message door for testing.",
      "times_run": 0,
      "last_run": null,
      "average_runtime_seconds": null
    },
    {
      "id": "fortune",
      "name": "Fortune Cookie",
      "description": "Random fortune messages",
      "command": "/usr/games/fortune",
      "arguments": ["-s"],
      "working_directory": null,
      "environment": {},
      "requires_login": false,
      "allowed_users": null,
      "min_user_level": null,
      "max_time_minutes": 2,
      "memory_limit_mb": 10,
      "category": "Entertainment",
      "author": "BSD Games",
      "version": null,
      "help_text": "Classic Unix fortune program. Displays random quotes, jokes, and sayings.",
      "times_run": 0,
      "last_run": null,
      "average_runtime_seconds": null
    },
    {
      "id": "adventure",
      "name": "Colossal Cave Adventure",
      "description": "Classic text adventure game",
      "command": "./doors/adventure/adventure", 
      "arguments": [],
      "working_directory": "./doors/adventure",
      "environment": {
        "ADVENT_DATA": "./adventure.dat"
      },
      "requires_login": true,
      "allowed_users": null,
      "min_user_level": null,
      "max_time_minutes": 60,
      "memory_limit_mb": 50,
      "category": "Game",
      "author": "William Crowther & Don Woods",
      "version": "2.5",
      "help_text": "The original computer adventure game. Explore the Colossal Cave, collect treasures, and solve puzzles. Type 'help' in game for commands.",
      "times_run": 0,
      "last_run": null,
      "average_runtime_seconds": null
    },
    {
      "id": "testdoor",
      "name": "Test Door",
      "description": "Simple test door for development",
      "command": "./doors/test/testdoor.sh",
      "arguments": [],
      "working_directory": "./doors/test",
      "environment": {},
      "requires_login": false,
      "allowed_users": null,
      "min_user_level": null,
      "max_time_minutes": 5,
      "memory_limit_mb": 10,
      "category": "Utility",
      "author": "BBS Development",
      "version": "1.0",
      "help_text": "Simple test door that displays environment variables and accepts user input.",
      "times_run": 0,
      "last_run": null,
      "average_runtime_seconds": null
    }
  ]
}
```

#### 7.2 Test Door Script
**File**: `doors/test/testdoor.sh`

```bash
#!/bin/bash
# Simple test door for BBS testing

echo "=== BBS Door Test Program ==="
echo "Node: $BBS_NODE"
echo "User: $BBS_USER"
echo "Terminal: ${BBS_TERM_WIDTH}x${BBS_TERM_HEIGHT}"
echo "ANSI: $BBS_TERM_ANSI, Color: $BBS_TERM_COLOR"
echo "Session ID: $BBS_SESSION_ID"
echo ""

echo "This is a test door. Type 'quit' to exit."
echo ""

while true; do
    echo -n "Test> "
    read input
    
    case "$input" in
        "quit"|"exit"|"q")
            echo "Goodbye!"
            exit 0
            ;;
        "info")
            echo "BBS: $BBS_NAME"
            echo "SysOp: $BBS_SYSOP" 
            echo "Your real name: $BBS_REALNAME"
            echo "Your location: $BBS_LOCATION"
            ;;
        "colors")
            if [ "$BBS_TERM_COLOR" = "true" ]; then
                echo -e "\033[31mRed\033[32m Green\033[34m Blue\033[0m Normal"
            else
                echo "Color not supported"
            fi
            ;;
        "help")
            echo "Commands: info, colors, quit"
            ;;
        *)
            echo "You said: $input"
            echo "Type 'help' for commands, 'quit' to exit"
            ;;
    esac
done
```

### Phase 8: Error Handling & Security

#### 8.1 Comprehensive Error Handling
- Process spawn failures
- PTY creation errors
- I/O bridging failures
- Timeout enforcement
- Resource limit enforcement
- Permission validation

#### 8.2 Security Measures
- User permission validation
- Resource limits (memory, time)
- Working directory restrictions
- Environment variable sanitization
- Process isolation

#### 8.3 Logging & Monitoring
- Door execution logs
- Performance statistics
- Error tracking
- Usage analytics

## 🧪 Testing Strategy

### Phase 1: Unit Tests
- Door configuration loading
- Permission validation
- Environment variable setup

### Phase 2: Integration Tests
- PTY creation and communication
- I/O bridging functionality
- Process lifecycle management

### Phase 3: End-to-End Tests
- Simple echo door
- Interactive test door
- Long-running door with timeout
- Permission denied scenarios

### Phase 4: Performance Tests
- Multiple concurrent doors
- Memory usage monitoring
- I/O throughput testing

## 📦 File Structure After Implementation

```
src/
├── doors.rs                    # Door data structures
├── door_repository.rs          # Door storage trait & implementation
├── door_runner.rs              # PTY-based door execution engine
├── menu/
│   └── menu_door.rs           # Door menu interface
├── services/
│   └── door_service.rs        # Door service layer
└── ... (existing files)

doors/
├── doors.json                  # Door definitions
├── test/
│   └── testdoor.sh            # Simple test door
└── adventure/                  # Example game door
    ├── adventure              # Game binary
    └── adventure.dat          # Game data

tests/
└── door_tests.rs              # Door system tests
```

## 🎯 Success Criteria

1. ✅ **Safe Implementation** - Zero unsafe code using `portable-pty`
2. ✅ **Cross-Platform** - Works on Unix, Linux, macOS, Windows
3. ✅ **Full Terminal Support** - Size, colors, ANSI sequences work correctly
4. ✅ **Robust Error Handling** - Graceful failure modes and recovery
5. ✅ **Security** - Proper permission validation and resource limits
6. ✅ **Performance** - Multiple concurrent doors without blocking BBS
7. ✅ **Maintainable** - Clean architecture integrated with existing codebase

This plan provides a comprehensive, safe, and professional implementation of BBS doors using modern Rust practices and the `portable-pty` crate for cross-platform terminal handling.