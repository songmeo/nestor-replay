# nestor-replay

Replay CAN bus recordings from [CyphalCloud](https://cyphalcloud.zubax.com) to SocketCAN (e.g. `vcan0`, `can0`).

By default (non-`--dry-run`), the tool avoids printing during replay to preserve timing and only prints a final summary line. Use `--dry-run` to print frames to stdout in a candump-style format.

## Installation

```bash
cargo build --release
```

Binary at `target/release/nestor-replay`.

## Requirements

Linux with SocketCAN. For testing with virtual CAN:

```bash
sudo modprobe vcan
sudo ip link add dev vcan0 type vcan
sudo ip link set up vcan0
```

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

### Non-dry-run (default)

To preserve replay timing, `nestor-replay` stays silent during replay and prints only a final summary line:

```
Replayed 10 frames in 0.2s
```

### Dry-run (`--dry-run`)

In `--dry-run` mode, frames are printed in a candump-style format (and a progress bar may be shown):

```
[   0.000s] vcan0  5A0 [8]  10 11 12 13 14 15 16 17
[   0.001s] vcan0  5A1 [8]  11 12 13 14 15 16 17 18
[   0.002s] vcan0  5A2 [8]  12 13 14 15 16 17 18 19
```

## CAN visualization (live, terminal)

This repo includes a terminal visualizer:

- `cargo run --bin can_viz -- --interface vcan0`

It listens on a SocketCAN interface (like `vcan0`) and shows a live per-ID table (rates, last payload, history).



## License

MIT
