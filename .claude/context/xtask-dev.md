# xtask dev Command

The `cargo xtask dev` command provides a unified interface for running the application in development mode across all platforms.

## Usage

```bash
cargo xtask dev <TARGET> [OPTIONS]
```

### Targets

| Target    | Description                      | Underlying Command          |
|-----------|----------------------------------|-----------------------------|
| `web`     | Run the Axum web server          | `cargo run -p calendsync`   |
| `desktop` | Run the Tauri desktop app        | `cargo tauri dev`           |
| `ios`     | Run the Tauri iOS app            | `cargo tauri ios dev`       |

## Web Options

```bash
cargo xtask dev web [OPTIONS]

Options:
  -p, --port <PORT>   Port to run on (default: 3000)
  --release           Build in release mode
```

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
# Web server
cargo xtask dev web                       # Port 3000
cargo xtask dev web --port 8080           # Custom port

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
├── mod.rs        # DevCommand, DevTarget enum, run()
├── error.rs      # DevError types
├── ios.rs        # iOS logic (device listing, launching)
├── web.rs        # Web server logic
└── desktop.rs    # Desktop app logic
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

1. **Unified interface** - One command instead of remembering three different invocations
2. **Proper `--device` flag** - iOS device is a named argument, not positional
3. **Device discovery** - `--list-devices` shows available simulators
4. **Consistent options** - `--release`, `--no-watch` work across all targets
5. **Extensible** - Easy to add Android support later
