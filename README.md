# Moonbase BBS (Bulletin Board System)

A traditional BBS implementation in Rust that accepts connections over SSH and Telnet.

## Project Goals
- Learn Rust network programming fundamentals
- Minimize external dependencies (except Crossterm/Ratatui for UI)
- Build incrementally with working software at each step
- Create something fun and nostalgic

## MVP Features

### Core Infrastructure
- [x] Project setup
- [ ] Telnet connection handling
- [ ] SSH connection handling (after Telnet works)
- [ ] Basic user session management
- [ ] Graceful connection cleanup

### User System
- [ ] User registration (username/password)
- [ ] Login system
- [ ] User data persistence (file-based)
- [ ] Basic user profiles

### Interface
- [ ] Main menu system with numbered options
- [ ] Navigation between areas
- [ ] Terminal UI with Crossterm
- [ ] ANSI art welcome screen

### BBS Features
- [ ] Bulletin/Message Board (post and read messages)
- [ ] User list (registered users and who's online)
- [ ] Basic messaging system (leave messages for users)
- [ ] File area (upload/download text files)

### System Features
- [ ] Basic logging (connections, user actions)
- [ ] Configuration system (port, welcome message, etc.)
- [ ] Graceful shutdown handling
- [ ] Multi-user concurrent access

## Build Order (To-Do List)

### Phase 1: Foundation
- [ ] 1. **Basic Telnet Connection** - Accept connections, send "Hello World"
- [ ] 2. **Menu System** - Build UI foundation with Crossterm
- [ ] 3. **User Registration/Login** - File-based user storage

### Phase 2: Core BBS Features
- [ ] 4. **Message Board** - Post and read bulletin messages
- [ ] 5. **User List** - Show registered and online users
- [ ] 6. **Basic Messaging** - User-to-user messages

### Phase 3: Enhanced Features
- [ ] 7. **File Area** - Simple file upload/download
- [ ] 8. **SSH Support** - Add SSH alongside Telnet
- [ ] 9. **System Administration** - Basic admin features
- [ ] 10. **Polish & Configuration** - Config files, better logging

## Learning Focus Areas
- TCP socket programming with `std::net`
- Concurrent programming (threads or async)
- Terminal control and ANSI escape sequences
- File I/O and data serialization
- Protocol implementation (Telnet negotiation)
- Session state management

## Dependencies
- `crossterm` - Terminal manipulation and input handling
- Standard library only for core networking
