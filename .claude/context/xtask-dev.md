# xtask dev Command

The `cargo xtask dev` command provides a unified interface for running the application in development mode across all platforms.

## Usage

```bash
cargo xtask dev <TARGET> [OPTIONS]
```

### Targets

| Target    | Description                      | Underlying Command          |
|-----------|----------------------------------|-----------------------------|
| `server`  | Run the development server       | `cargo run -p calendsync`   |
| `desktop` | Run the Tauri desktop app        | `cargo tauri dev`           |
| `ios`     | Run the Tauri iOS app            | `cargo tauri ios dev`       |

## Server Options

The server target supports configurable storage and cache backends with automatic container orchestration:

```bash
cargo xtask dev server [OPTIONS]

Storage/Cache:
  --storage <TYPE>     inmemory (default), sqlite, dynamodb
  --cache <TYPE>       memory (default), redis

Container:
  --podman             Use podman instead of docker
  --flush              Remove volumes before starting containers

Data:
  --seed               Seed database with demo data via HTTP

Other:
  -p, --port <PORT>    Port to run on (default: 3000)
  --release            Build in release mode
  --no-hot-reload      Disable TypeScript hot-reload
  --no-auto-refresh    Disable browser auto-refresh
  --keep-containers    Keep containers running on error
```

For detailed documentation on the server command, see `.claude/context/dev-server.md`.

## Desktop Options

```bash
cargo xtask dev desktop [OPTIONS]

Options:
  -t, --target <TRIPLE>   Target triple to build against
  --release               Build in release mode
  --no-watch              Disable file watching
```

## iOS Options

```bash
cargo xtask dev ios [OPTIONS]

Options:
  -d, --device <NAME>   iOS simulator or device name (e.g., "iPhone 16 Pro")
  --list-devices        List available iOS simulators with boot status
  -o, --open            Open Xcode instead of running directly
  --host <IP>           Use public network address for physical devices
  --release             Build in release mode
  --no-watch            Disable file watching
```

## Examples

```bash
# Development server
cargo xtask dev server                              # Default: inmemory + memory
cargo xtask dev server --seed                       # With demo data
cargo xtask dev server --storage sqlite --seed      # SQLite storage
cargo xtask dev server --storage dynamodb --seed    # DynamoDB (auto-starts container)
cargo xtask dev server --cache redis --seed         # Redis cache (auto-starts container)
cargo xtask dev server --storage dynamodb --cache redis --seed  # Full stack

# Desktop app
cargo xtask dev desktop
cargo xtask dev desktop --release

# iOS app
cargo xtask dev ios                       # Default simulator
cargo xtask dev ios --list-devices        # Shows ● booted, ○ shutdown
cargo xtask dev ios --device "iPhone 16"  # Specific device
cargo xtask dev ios --open                # Opens Xcode
cargo xtask dev ios --host                # Physical device
```

## Architecture

```
xtask/src/dev/
├── mod.rs          # DevCommand, DevTarget enum, run()
├── error.rs        # DevError types
├── server.rs       # Server logic with container orchestration
├── containers.rs   # Container management (Docker/Podman)
├── seed.rs         # HTTP-based data seeding
├── desktop.rs      # Desktop app logic
└── ios.rs          # iOS logic (device listing, launching)
```

## iOS Device Listing

The `--list-devices` flag shows available simulators with their boot status:

```
Available iOS Simulators:

-- iOS 18.6 --
  ● iPhone 16 Pro (F0FDEDA7-...) (Booted)     # Currently running
  ○ iPhone 16 (4643120C-...) (Shutdown)        # Available but not running
  ○ iPad Pro 11-inch (M4) (4F6AE226-...) (Shutdown)
```

## Why This Exists

1. **Unified interface** - One command instead of remembering different invocations
2. **Container orchestration** - Automatic Docker/Podman management for DynamoDB and Redis
3. **HTTP-based seeding** - Demo data via API endpoints validates the full stack
4. **Device discovery** - `--list-devices` shows available iOS simulators
5. **Consistent options** - `--release`, `--seed` work across configurations
6. **Extensible** - Easy to add Android support later
