# AGENTS.md - Audio Station Coding Guidelines

## Build & Test Commands

### Build
```sh
cargo build              # Debug build
cargo build --release    # Release build with optimizations
```

### Run
```sh
cargo run -- client      # Run client mode
cargo run -- server      # Run server mode
```

### Test
```sh
cargo test                          # Run all tests
cargo test test_name                # Run single test by name
cargo test --lib                    # Run library tests only
cargo test -- --nocapture           # Run tests with stdout visible
cargo test package_name             # Run tests for specific package
```

### Lint
```sh
cargo clippy                        # Run Clippy linter
cargo clippy -- -D warnings         # Deny all warnings
cargo clippy --fix                  # Auto-fix lint suggestions
cargo fmt                           # Format code with rustfmt
cargo fmt --check                   # Check formatting without changes
```

### Check
```sh
cargo check                         # Fast compilation check
cargo check --all-targets           # Check all targets
```

## Code Style Guidelines

### Imports
- Group imports by source: std, external crates, local modules
- Use `use crate::` for local module imports
- Sort imports alphabetically within groups

### Formatting
- Indentation: 4 spaces (no tabs)
- Max line length: 100 characters preferred
- Use `cargo fmt` before committing
- Blank lines between function definitions

### Naming Conventions
- **Types/Structs/Enums**: PascalCase (e.g., `SocketAddr`, `Commands`)
- **Functions/Methods**: snake_case (e.g., `run()`, `get_interfaces()`)
- **Variables**: snake_case (e.g., `broadcast_addr`, `encoder_clone`)
- **Constants**: UPPER_SNAKE_CASE (e.g., `BROADCAST_PORT`, `MAGIC_HEADER`)
- **Traits**: PascalCase with descriptive names (e.g., `DeviceTrait`, `StreamTrait`)

### Error Handling
- Return `io::Result<T>` from functions that can fail
- Use `map_err()` to convert library errors to `io::Error`
- Use `ok_or_else()` for Option to Result conversion
- Log errors with `eprintln!()` for non-fatal errors
- Error kinds: `NotFound`, `Other`, `InvalidInput`

### Types & Type Safety
- Use explicit type annotations for clarity
- Prefer `Arc<Mutex<T>>` for shared mutable state
- Use `Arc<AtomicBool>` for thread-safe flags
- Use `#[derive(Parser)]` and `#[derive(Subcommand)]` for CLI

### Documentation
- No inline comments unless explaining non-obvious logic
- Use doc comments (`///`) for public APIs if needed
- Add println!() for user-facing status messages

### Performance
- Reuse buffers with `thread_local!` to avoid allocations
- Use fixed buffer sizes where possible (e.g., `[u8; SIZE]`)
- Avoid cloning in hot paths
- Use `&[T]` slices instead of `Vec<T>` for read-only data

### Logging & Output
- Use `println!()` for user-facing info messages
- Use `eprintln!()` for errors and warnings
- Include context in error messages (addresses, names, values)

### Dependencies
- `cpal = "0.15"` - Audio I/O
- `clap = "4.4"` - CLI parsing  
- `opus2 = "0.3.3"` - Audio codec
- `ringbuf = "0.4"` - Ring buffer
- `if-addrs = "0.13"` - Network interfaces
- `bytemuck = "1.14"` - Type casting

### Platform Considerations
- Cross-platform: Linux, Windows, macOS
- Windows linking: `x86_64-w64-mingw32-gcc` (see `.cargo/config.toml`)
- Use `Ipv4Addr::UNSPECIFIED` for binding to all interfaces

### Git & Commits
- Do not commit unless explicitly requested
- Run `cargo clippy` and `cargo fmt` before commits
- Ensure tests pass before committing
- Commit messages should explain "why" not "what"

### Common Patterns

#### Module Entry Point
```rust
pub fn run() -> io::Result<()> {
    // module logic
}
```

#### Thread-Safe Shared State
```rust
let shared = Arc::new(Mutex::new(value));
let shared_clone = shared.clone();
```

#### Error Conversion
```rust
.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
```

#### Buffer Reuse
```rust
thread_local! {
    static BUFFER: RefCell<[u8; SIZE]> = RefCell::new([0u8; SIZE]);
}
```

## Project Structure
```
audio_station/
├── Cargo.toml           # Package manifest
├── .cargo/config.toml   # Platform-specific config
├── src/
│   ├── main.rs          # CLI entry point
│   ├── client.rs        # Client mode implementation
│   ├── server.rs        # Server mode implementation
│   └── shared.rs        # Shared constants & utilities
```

## Architecture
- **Client**: Captures audio → OPUS encodes → UDP broadcast
- **Server**: UDP receive → OPUS decode → Audio playback
- **Protocol**: Custom header + length-prefixed OPUS packets
- **Sample Rate**: Fixed 48kHz stereo
- **Network**: IPv4 broadcast on port 5000
