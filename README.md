# Audio Station

Play audio from multiple devices through one set of speakers.

Audio Station captures audio from your device, encodes it with the OPUS codec, and broadcasts it over the network. A server can receive streams from multiple clients and play them all through a single output device.

## Features

- **Multi-device audio**: Combine audio from multiple computers into one output
- **Low latency**: OPUS codec provides high-quality audio with minimal delay
- **Network broadcast**: Uses IPv4 broadcast to automatically discover clients
- **Cross-platform**: Works on Linux, Windows, and macOS
- **48kHz stereo**: High-quality audio at 48kHz sample rate

## Requirements

- Rust toolchain (for building from source)
- Audio devices (input for clients, output for server)
- Network connectivity (UDP port 5000)

## Installation

```sh
cargo build --release
```

The binary will be available at `target/release/audio_station`.

## Usage

### Global Options

- `--port <PORT>` - UDP port for broadcast (default: 5000)

### Client Mode

Capture and broadcast audio from this device:

```sh
audio_station client
```

Options:
- `--type <TYPE>` - Audio source: `input` (microphone) or `output` (system audio/speakers) (default: `output`)

### Server Mode

Receive and play broadcasted audio:

```sh
audio_station server
```

### Examples

Use a custom port for both client and server:

```sh
audio_station --port 6000 client
audio_station --port 6000 server
```

## How It Works

1. **Client**: Captures audio → OPUS encodes → UDP broadcast
2. **Server**: UDP receive → OPUS decode → Audio playback

Multiple clients can broadcast simultaneously, and the server will mix all received streams.

## Protocol

- Custom header with magic bytes for packet identification
- Length-prefixed OPUS packets
- IPv4 broadcast (default port: 5000)
- Fixed 48kHz stereo output

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

## Development

### Build

```sh
cargo build              # Debug build
cargo build --release    # Release build
```

### Run

```sh
cargo run -- client      # Run client mode
cargo run -- server      # Run server mode
```

### Test

```sh
cargo test                          # Run all tests
cargo test -- --nocapture           # Run tests with stdout visible
```

### Lint & Format

```sh
cargo clippy                        # Run linter
cargo fmt                           # Format code
```

## Dependencies

- `cpal` - Cross-platform audio I/O
- `opus2` - OPUS audio codec
- `clap` - Command-line argument parsing
- `ringbuf` - Lock-free ring buffer for audio data
- `if-addrs` - Network interface discovery
- `bytemuck` - Zero-copy type conversions

## License

GPL-2.0
