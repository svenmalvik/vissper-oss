# macOS Menu Bar Application - Technology Stack & Architecture

## Core Technology Stack

### Language & Runtime
- **Rust 2021 Edition** - Systems programming language for performance and safety
- **Tokio** (async runtime) - Full async/await support for all I/O operations
- **Main Thread Pattern** - Critical for macOS AppKit integration

### macOS Native Integration (via objc2, NOT Swift)

**Important Note**: This application uses **Rust with objc2 bindings** directly to Objective-C/AppKit, not Swift. objc2 provides zero-cost Rust bindings to macOS frameworks.

#### Key objc2 Crates:
```toml
objc2 = "0.5"
objc2-foundation = { version = "0.2", features = ["NSData", "NSString", "NSThread", "NSObject", "NSOperation", "NSAttributedString", "NSRange", "NSDictionary"] }
objc2-app-kit = { version = "0.2", features = [
    "NSApplication",
    "NSMenu", "NSMenuItem",
    "NSStatusBar", "NSStatusBarButton", "NSStatusItem",
    "NSWindow", "NSView", "NSTextField", "NSButton",
    # ... add features as needed for your UI components
] }
block2 = "0.5"              # Objective-C blocks support
dispatch = "0.2"            # GCD (Grand Central Dispatch) for main thread operations
security-framework = "2.9"  # macOS Keychain for secure credential storage
```

### Application Architecture Pattern

#### Application Type: Menu Bar Only (NSApplicationActivationPolicy::Accessory)
- No dock icon
- Lives in menu bar (status bar)
- Perfect for background utilities and monitoring apps

#### Thread Safety Model

**CRITICAL PATTERN**: All AppKit operations MUST run on the main thread:

```rust
use objc2_foundation::MainThreadMarker;
use dispatch::Queue;

pub fn show_window() {
    // Check if already on main thread
    if let Some(mtm) = MainThreadMarker::new() {
        show_window_on_main_thread(mtm);
        return;
    }

    // Not on main thread - dispatch to main
    Queue::main().exec_async(|| {
        if let Some(mtm) = MainThreadMarker::new() {
            show_window_on_main_thread(mtm);
        }
    });
}

fn show_window_on_main_thread(mtm: MainThreadMarker) {
    // Safe to call AppKit APIs here
    // MainThreadMarker proves we're on main thread
}
```

#### Global State Management Pattern

Uses `OnceCell` for lazy initialization of singleton components:

```rust
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};
use objc2::rc::Retained;

// Global state pattern
static MENU_BAR: OnceCell<Mutex<MenuBarInner>> = OnceCell::new();
static APP_STATE: OnceCell<Arc<AppState>> = OnceCell::new();
static CALLBACKS: OnceCell<MenuCallbacks> = OnceCell::new();

pub struct MenuBarInner {
    // All Objective-C objects use Retained<T> for memory management
    status_item: Retained<NSStatusItem>,
    menu: Retained<NSMenu>,
    menu_items: Vec<Retained<NSMenuItem>>,
}

// Required for cross-thread access to Objective-C objects
unsafe impl Send for MenuBarInner {}
```

### Objective-C Delegate Pattern in Rust

Define custom Objective-C classes in Rust using `declare_class!`:

```rust
use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_foundation::{NSObject, NSObjectProtocol};

declare_class!(
    pub struct MyMenuDelegate;

    unsafe impl ClassType for MyMenuDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "MyMenuDelegate";
    }

    impl DeclaredClass for MyMenuDelegate {}

    unsafe impl MyMenuDelegate {
        // Define Objective-C methods
        #[method(handleAction:)]
        fn handle_action(&self, _sender: *mut NSObject) {
            // Rust implementation of action handler
            if let Some(callbacks) = CALLBACKS.get() {
                (callbacks.on_action)();
            }
        }
    }

    unsafe impl NSObjectProtocol for MyMenuDelegate {}
);
```

### Menu Bar Construction Pattern

```rust
pub fn init(state: Arc<AppState>, callbacks: MenuCallbacks) {
    let mtm = MainThreadMarker::new()
        .expect("init() must be called on main thread");

    // Get shared application
    let app = NSApplication::sharedApplication(mtm);

    // Set as menu bar only (no dock icon)
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    // Create status bar item
    let status_bar = unsafe { NSStatusBar::systemStatusBar() };
    let status_item = unsafe { status_bar.statusItemWithLength(-2.0) }; // NSVariableStatusItemLength

    // Create delegate
    let delegate = MyMenuDelegate::new(mtm);

    // Create menu
    let menu = NSMenu::new(mtm);
    unsafe { menu.setAutoenablesItems(false) }; // Manual control over enabled state

    // Create menu items
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            mtm.alloc(),
            &NSString::from_str("My Action"),
            Some(sel!(handleAction:)),
            &NSString::from_str(""), // Keyboard shortcut (empty = none)
        )
    };
    item.setTarget(Some(&delegate));
    menu.addItem(&item);

    // Attach menu to status item
    unsafe { status_item.setMenu(Some(&menu)) };

    // Store in global state
    MENU_BAR.set(Mutex::new(MenuBarInner {
        status_item,
        menu,
        delegate,
        // ... retain all objects that need to stay alive
    })).ok();
}

pub fn run() {
    let mtm = MainThreadMarker::new()
        .expect("run() must be called on main thread");
    let app = NSApplication::sharedApplication(mtm);
    unsafe { app.run() }; // Blocks until app quits
}
```

### Standard Edit Menu for Keyboard Shortcuts

Menu bar apps need a main menu for standard keyboard shortcuts (Cmd+C, Cmd+V, etc.):

```rust
fn setup_edit_menu(mtm: MainThreadMarker, app: &NSApplication) {
    let main_menu = NSMenu::new(mtm);
    let edit_menu = NSMenu::new(mtm);
    unsafe { edit_menu.setTitle(&NSString::from_str("Edit")) };

    // Add standard items (Copy, Paste, Cut, Select All, etc.)
    let copy_item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            mtm.alloc(),
            &NSString::from_str("Copy"),
            Some(sel!(copy:)),
            &NSString::from_str("c"),
        )
    };
    edit_menu.addItem(&copy_item);

    // ... more items ...

    let edit_menu_item = NSMenuItem::new(mtm);
    edit_menu_item.setSubmenu(Some(&edit_menu));
    main_menu.addItem(&edit_menu_item);

    unsafe {
        let _: () = msg_send![app, setMainMenu: &*main_menu];
    }
}
```

### Callback-Based Architecture

Decouple UI from business logic using callback pattern:

```rust
pub struct MenuCallbacks {
    pub on_action: Arc<dyn Fn() + Send + Sync>,
    pub on_settings: Arc<dyn Fn() + Send + Sync>,
    pub on_quit: Arc<dyn Fn() + Send + Sync>,
}

// In main.rs
let callbacks = MenuCallbacks {
    on_action: Arc::new(|| {
        tokio::spawn(async {
            // Async business logic here
        });
    }),
    on_settings: Arc::new(|| {
        settings_window::show();
    }),
    on_quit: Arc::new(|| {
        // Cleanup
    }),
};

menubar::MenuBar::init(app_state, callbacks);
```

## Essential Dependencies

### Async & Networking
```toml
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
tokio-tungstenite = { version = "0.21", features = ["native-tls"] }  # WebSocket
futures-util = "0.3"
```

### Serialization & Configuration
```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
```

### Error Handling & Logging
```toml
anyhow = "1.0"      # Application-level errors with context
thiserror = "1.0"   # Custom error types
tracing = "0.1"     # Structured logging
tracing-subscriber = "0.3"
```

### Platform Integration
```toml
global-hotkey = "0.6"  # Global keyboard shortcuts
arboard = "3.4"        # Cross-platform clipboard
dirs = "5.0"           # User directories (Documents, etc.)
open = "5.0"           # Open URLs in browser
```

### Security
```toml
zeroize = "1.7"  # Secure memory clearing for secrets
```

## Key Architectural Principles

### 1. Main Thread Discipline
- **All AppKit calls must use `MainThreadMarker`**
- Use `dispatch::Queue::main().exec_async()` to dispatch from background threads
- Never call UI methods from Tokio tasks directly

### 2. Memory Management with Retained<T>
- All NSObject references use `Retained<T>` (from objc2)
- Automatic reference counting (ARC) semantics
- Objects stored in global state must be explicitly retained

### 3. Unsafe Block Discipline
- Use `unsafe` only for FFI calls to AppKit/Foundation
- Document safety invariants
- Keep unsafe blocks minimal

### 4. Error Handling Pattern
```rust
use anyhow::{Context, Result};

fn do_something() -> Result<()> {
    let data = read_file()
        .with_context(|| "Failed to read configuration file")?;

    process_data(data)
        .context("Failed to process data")?;

    Ok(())
}
```

### 5. Logging Best Practices
```rust
use tracing::{info, warn, error, debug};

// In main.rs
tracing_subscriber::fmt::init();

// Throughout code
info!("Application started");
debug!("Processing item: {:?}", item);
error!("Failed to connect: {}", err);

// NEVER log sensitive data (credentials, PII, etc.)
```

### 6. Configuration Management
```rust
// Embed config at compile time
const CONFIG_TOML: &str = include_str!("../config.toml");

#[derive(serde::Deserialize)]
struct Config {
    app_name: String,
    version_check: VersionCheckConfig,
}

fn load_config() -> Result<Config> {
    let config: Config = toml::from_str(CONFIG_TOML)?;
    Ok(config)
}
```

### 7. Secure Credential Storage (macOS Keychain)
```rust
use security_framework::passwords::*;

const SERVICE_NAME: &str = "com.yourapp.desktop";

fn store_credentials(api_key: &str) -> Result<()> {
    let _ = delete_generic_password(SERVICE_NAME, "api_key");
    set_generic_password(SERVICE_NAME, "api_key", api_key.as_bytes())
        .context("Failed to store credentials")?;
    Ok(())
}

fn get_credentials() -> Result<String> {
    let password = get_generic_password(SERVICE_NAME, "api_key")
        .context("Failed to retrieve credentials")?;

    String::from_utf8(password.to_vec())
        .context("Invalid UTF-8 in stored credentials")
}
```

## Project Structure

```
src/
├── main.rs                 # Entry point, initialization
├── menubar/               # Menu bar implementation
│   ├── mod.rs            # MenuBar struct, init, run
│   ├── delegate.rs       # Objective-C delegate class
│   ├── builder.rs        # Menu construction
│   ├── state.rs          # AppState, MenuCallbacks
│   └── updates/          # Dynamic UI updates
├── callbacks/            # Business logic callbacks
├── settings_window/      # Settings UI (NSWindow)
├── keychain.rs          # Secure credential storage
├── hotkeys.rs           # Global keyboard shortcuts
└── error.rs            # Custom error types
```

## Common Patterns

### Dynamic Menu Updates (Thread-Safe)
```rust
pub fn set_recording(recording: bool) {
    // Dispatch to main thread if needed
    dispatch::Queue::main().exec_async(move || {
        if let Some(mtm) = MainThreadMarker::new() {
            set_recording_on_main_thread(recording, mtm);
        }
    });
}

fn set_recording_on_main_thread(recording: bool, _mtm: MainThreadMarker) {
    if let Some(menu_bar) = MENU_BAR.get() {
        let inner = menu_bar.lock().unwrap();
        let title = if recording { "Stop" } else { "Start" };
        unsafe {
            inner.recording_item.setTitle(&NSString::from_str(title));
        }
    }
}
```

### Global Hotkeys
```rust
use global_hotkey::{hotkey::{Code, HotKey, Modifiers}, GlobalHotKeyManager};

fn init_hotkeys() -> Result<GlobalHotKeyManager> {
    let manager = GlobalHotKeyManager::new()?;

    // Cmd + Shift + 1
    let hotkey = HotKey::new(
        Some(Modifiers::CONTROL | Modifiers::SHIFT),
        Code::Digit1
    );
    manager.register(hotkey)?;

    Ok(manager)
}

// Listener runs on dedicated thread (not Tokio)
fn start_listener(callback: Arc<dyn Fn() + Send + Sync>) {
    std::thread::spawn(move || {
        let receiver = GlobalHotKeyEvent::receiver();
        loop {
            if let Ok(event) = receiver.try_recv() {
                let cb = callback.clone();
                dispatch::Queue::main().exec_async(move || {
                    (cb)();
                });
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    });
}
```

## Swift vs objc2 Comparison

**This project uses objc2 (Rust → Objective-C), NOT Swift.**

### If you want to use Swift instead:

You would need:
1. Create a Swift package with Xcode
2. Build the Swift code as a dynamic library or framework
3. Use FFI (foreign function interface) to call from Rust
4. Significantly more complex build process

### Why objc2 is preferred here:
- **Single language**: Everything in Rust
- **Zero-cost bindings**: Direct Objective-C calls, no overhead
- **Type safety**: Rust's type system + AppKit types
- **Simpler build**: Just `cargo build`, no Xcode required
- **Direct control**: Fine-grained control over memory and threading

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_function() {
        // Regular unit test
    }

    #[tokio::test]
    async fn test_async_function() {
        // Async unit test
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_specific() {
        // macOS-only test
    }
}
```

## Build & Run

```bash
# Development build
cargo build

# Development run
cargo run

# Release build
cargo build --release

# Run tests
cargo test

# Linting
cargo clippy --all-targets --all-features

# Formatting
cargo fmt
```

## Summary for New Project

When creating a new macOS menu bar application with Rust:

1. **Use objc2** for native macOS integration (not Swift unless you have specific requirements)
2. **Respect main thread requirements** - use `MainThreadMarker` everywhere
3. **Use Retained<T>** for all NSObject references
4. **Adopt callback pattern** to decouple UI from business logic
5. **Use OnceCell** for global singleton state
6. **Implement Objective-C delegates** with `declare_class!` macro
7. **Use macOS Keychain** for secure credential storage
8. **Set activation policy to Accessory** for menu bar-only apps
9. **Add Edit menu** for standard keyboard shortcuts to work
10. **Use Tokio** for async operations, but always dispatch UI updates to main thread

## Additional Resources

- [objc2 Documentation](https://docs.rs/objc2/)
- [objc2-foundation Documentation](https://docs.rs/objc2-foundation/)
- [objc2-app-kit Documentation](https://docs.rs/objc2-app-kit/)
- [Apple AppKit Documentation](https://developer.apple.com/documentation/appkit)
- [Tokio Documentation](https://tokio.rs/)
- [Rust Async Book](https://rust-lang.github.io/async-book/)
