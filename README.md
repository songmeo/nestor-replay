# nestor-replay

Replay CAN bus recordings from [CyphalCloud](https://cyphalcloud.zubax.com)/[Nestor](https://github.com/Zubax/nestor) server to SocketCAN.

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
# Binary at: target/release/nestor-replay
```

## Usage

```bash
# Replay from CyphalCloud (default server)
nestor-replay --device my-device --boot 0

# Replay with custom speed (2x faster)
nestor-replay --device my-device --boot 0 --speed 2.0

# Use different SocketCAN interface
nestor-replay --device my-device --boot 0 --interface can0

# Dry run (display frames without sending to CAN)
nestor-replay --device my-device --boot 0 --dry-run

# Use local Nestor server
nestor-replay --server http://localhost:8000 --device my-device --boot 0
```

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `-s, --server <URL>` | Nestor server URL | `https://cyphalcloud.zubax.com` |
| `-d, --device <NAME>` | Device name | (required) |
| `-b, --boot <ID>` | Boot session ID | (required) |
| `-i, --interface <NAME>` | SocketCAN interface | `vcan0` |
| `--speed <MULTIPLIER>` | Playback speed | `1.0` |
| `--dry-run` | Display frames without sending | off |

## Output

Frames are displayed in candump-style format as they're replayed:

```
[   0.000s] vcan0  5A0 [8]  10 11 12 13 14 15 16 17
[   0.001s] vcan0  5A1 [8]  11 12 13 14 15 16 17 18
[   0.002s] vcan0  5A2 [8]  12 13 14 15 16 17 18 19
```

## Requirements

- Linux with SocketCAN support
- Virtual CAN interface for testing:
  ```bash
  sudo modprobe vcan
  sudo ip link add dev vcan0 type vcan
  sudo ip link set up vcan0
  ```

## API

nestor-replay uses the [Nestor REST API](https://cyphalcloud.zubax.com/docs):

- `GET /cf3d/api/v1/devices` — List devices
- `GET /cf3d/api/v1/boots?device=<name>` — List boot sessions
- `GET /cf3d/api/v1/records?device=<name>&boot_id=<id>` — Fetch CAN records

## License

MIT
