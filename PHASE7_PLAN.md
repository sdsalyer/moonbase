# Phase 7: Moonbase BBS Enhanced Telnet Integration

## Overview
Phase 7 will demonstrate the real-world value of telnet option negotiation by enhancing the Moonbase BBS user experience with intelligent terminal capability detection and adaptive features.

## Prerequisites  
- Phase 6 must be completed (Echo, Terminal Type, NAWS options implemented)
- Current TelnetStream integration from Phase 5

## Implementation Plan

### 1. Password Security Enhancement

#### 1.1 Echo Option Integration
- **Location**: `src/session.rs` - login/registration functions
- **Implementation**: Use Echo option negotiation for secure password input
- **Process**:
  1. Before password prompt: Negotiate Echo OFF  
  2. During password input: Terminal won't echo keystrokes
  3. After password input: Restore Echo ON

#### 1.2 Security Features
```rust
// Enhanced password input with echo control  
fn secure_password_input(&mut self, stream: &mut TelnetStream, prompt: &str) -> BbsResult<String> {
    // Disable echo for password security
    stream.request_echo_off()?;
    
    let password = self.get_input(stream, prompt)?;
    
    // Re-enable echo after password input
    stream.request_echo_on()?;
    
    Ok(password)
}
```

### 2. Smart Terminal Detection

#### 2.1 Terminal Width Auto-Detection  
- **Option**: NAWS (Negotiate About Window Size - RFC 1073)
- **Location**: `src/session.rs`, `src/config.rs`
- **Features**:
  - Detect client terminal width automatically
  - Responsive menu rendering based on actual terminal size
  - Fallback to configured default for non-supporting clients

#### 2.2 Terminal Capability Detection
- **Option**: Terminal Type (RFC 1091) 
- **Location**: `src/session.rs`, `src/box_renderer.rs`
- **Features**:
  - Detect ANSI support capabilities
  - Detect color support (256-color, 24-bit, etc.)
  - Adaptive UI rendering based on capabilities

#### 2.3 Implementation Structure
```rust
pub struct TerminalCapabilities {
    pub width: Option<u16>,
    pub height: Option<u16>, 
    pub terminal_type: Option<String>,
    pub supports_ansi: bool,
    pub supports_color: bool,
    pub color_depth: ColorDepth,
}

enum ColorDepth {
    Monochrome,
    Basic8,      // Basic 8 colors  
    Extended256, // 256 color palette
    TrueColor,   // 24-bit RGB
}
```

### 3. Configuration Enhancement

#### 3.1 New Configuration Options
**File**: `bbs.conf`
```toml
[ui]
# Existing options (maintained for compatibility)
box_style = "ascii" 
use_colors = false
menu_width = 80

# New Phase 7 auto-detection options
terminal_width = "auto"      # "auto" or specific number
ansi_support = "auto"        # "auto", "true", "false"  
color_support = "auto"       # "auto", "true", "false"
adaptive_layout = true       # Enable responsive design
fallback_width = 80          # Fallback when auto-detection fails
```

#### 3.2 Configuration Loading
- **Location**: `src/config.rs`
- **Enhancement**: Add new fields to `BbsConfig` struct
- **Validation**: Ensure backward compatibility with existing configs

### 4. Adaptive UI Rendering

#### 4.1 Box Renderer Enhancement
- **Location**: `src/box_renderer.rs`
- **Features**:
  - Dynamic width adjustment based on terminal capabilities
  - Smart box style selection (Unicode → ASCII fallback)
  - Adaptive color theme selection

#### 4.2 Menu System Enhancement  
- **Location**: `src/menu/` modules
- **Features**:
  - Responsive menu layouts
  - Dynamic column adjustment for wide terminals
  - Graceful degradation for narrow terminals

### 5. Session Enhancement

#### 5.1 Capability Negotiation
```rust
impl BbsSession {
    fn negotiate_terminal_capabilities(&mut self, stream: &mut TelnetStream) -> BbsResult<TerminalCapabilities> {
        let mut caps = TerminalCapabilities::default();
        
        // Request window size information
        if let Some((width, height)) = stream.request_window_size()? {
            caps.width = Some(width);
            caps.height = Some(height);
        }
        
        // Request terminal type
        if let Some(terminal_type) = stream.request_terminal_type()? {
            caps.terminal_type = Some(terminal_type.clone());
            caps.supports_ansi = Self::detect_ansi_support(&terminal_type);
            caps.supports_color = Self::detect_color_support(&terminal_type);
        }
        
        Ok(caps)
    }
}
```

#### 5.2 Session Startup Enhancement
1. Establish TelnetStream connection (Phase 5 - ✅ Complete)
2. **NEW**: Negotiate terminal capabilities  
3. **NEW**: Configure adaptive UI based on capabilities
4. Initialize terminal and show welcome (existing)
5. Continue with normal BBS flow

### 6. Testing Strategy

#### 6.1 Unit Tests
- Terminal capability detection logic
- Configuration parsing for new options
- Responsive layout calculations  
- Echo option negotiation

#### 6.2 Integration Tests
- Test with various terminal emulators:
  - Basic telnet (minimal capabilities)
  - Modern terminals (PuTTY, Terminal.app, etc.)
  - Different window sizes
  - Different color capabilities

#### 6.3 User Experience Tests  
- Password masking verification
- Responsive layout verification  
- Graceful fallback verification

### 7. Implementation Order

1. **Phase 6 Prerequisites** (Echo, Terminal Type, NAWS options)
2. **Configuration Enhancement** (new config options)
3. **Terminal Capability Detection** (negotiation logic)
4. **Password Security** (echo control for secure input)
5. **Adaptive UI** (responsive rendering)
6. **Testing & Polish** (comprehensive testing across terminals)

### 8. Backward Compatibility

- All existing `bbs.conf` files will continue to work unchanged
- New auto-detection features are opt-in via configuration
- Fallback behavior matches current static configuration
- No breaking changes to existing BBS functionality

### 9. Success Criteria

✅ **Password Security**: Login passwords are properly masked using telnet echo negotiation  
✅ **Smart Width**: Terminal width is auto-detected and menus adapt responsively  
✅ **Smart Colors**: Color themes activate only on capable terminals  
✅ **Smart ANSI**: Box rendering adapts to terminal ANSI capabilities  
✅ **Configuration**: New auto-detection options work alongside existing settings  
✅ **Compatibility**: Legacy terminals and configs continue working unchanged  
✅ **Testing**: Comprehensive test coverage across terminal types

## Expected Impact

Phase 7 will transform the Moonbase BBS from a static terminal application to an intelligent, adaptive system that provides an optimal experience across diverse client terminals while maintaining the nostalgic BBS aesthetic.

This phase will serve as a compelling demonstration of why proper telnet option negotiation matters in real-world applications.