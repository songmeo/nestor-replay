# nestor-replay

Replay CAN bus recordings from [CyphalCloud](https://cyphalcloud.zubax.com) to SocketCAN.

## Installation

```bash
cargo build --release
```

Binary at `target/release/nestor-replay`.

## Usage

```bash
nestor-replay [OPTIONS]
```

### Options

```
-s, --server <URL>       Nestor server URL [default: https://cyphalcloud.zubax.com]
-d, --device <NAME>      Device name (skip interactive selection)
-b, --boot <ID>          Boot session ID (skip interactive selection)
-i, --interface <NAME>   SocketCAN interface [default: vcan0]
    --speed <MULT>       Playback speed multiplier [default: 1.0]
    --dry-run            Display frames without sending to CAN
```

### Examples

```bash
# Replay a specific boot session
nestor-replay --device my-device --boot 0

# Replay at 2x speed
nestor-replay --device my-device --boot 0 --speed 2.0

# Dry run (display only, no CAN output)
nestor-replay --device my-device --boot 0 --dry-run

# Use local Nestor server
nestor-replay --server http://localhost:8000 --device my-device --boot 0
```

## Output

Frames displayed in candump-style format:

```
[   0.000s] vcan0  5A0 [8]  10 11 12 13 14 15 16 17
[   0.001s] vcan0  5A1 [8]  11 12 13 14 15 16 17 18
[   0.002s] vcan0  5A2 [8]  12 13 14 15 16 17 18 19
```

## Requirements

Linux with SocketCAN. For testing with virtual CAN:

```bash
sudo modprobe vcan
sudo ip link add dev vcan0 type vcan
sudo ip link set up vcan0
```

## License

MIT
