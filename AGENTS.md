# nestor-replay

Rust CLI tool to replay CAN bus recordings from CyphalCloud/Nestor server.

## Requirements

Build an **interactive TUI** that:

1. **Connects to CyphalCloud API** (https://cyphalcloud.zubax.com/cf3d/api/v1/)
2. **Lists devices** → user selects one
3. **Lists boots** (recording sessions) for that device → user selects one or more
4. **Fetches CAN records** from selected boots
5. **Replays to SocketCAN** (vcan0 by default) with correct timing from `hw_ts_us`
6. **Shows frames as they replay** (like candump output)

## CyphalCloud API

Base URL: `https://cyphalcloud.zubax.com`

### Endpoints

```
GET /cf3d/api/v1/devices
  → { "devices": [{ "device": "alpha", "last_heard_ts": 123, "last_uid": 456 }] }

GET /cf3d/api/v1/boots?device=<name>
  → { "device": "alpha", "boots": [{ "boot_id": 1, "first_record": {...}, "last_record": {...} }] }

GET /cf3d/api/v1/records?device=<name>&boot_id=<id>&limit=10000
  → { "device": "alpha", "records": [{ "hw_ts_us": 100, "boot_id": 1, "seqno": 10, "frame": {...} }] }
```

### CAN Frame format

```json
{
  "can_id": 291,
  "extended": false,
  "rtr": false,
  "error": false,
  "data_hex": "aabb"
}
```

## CLI Usage

```bash
# Interactive mode (default)
nestor-replay
# Shows: device list → boot list → replay options → replay with live output

# Direct mode (skip TUI)
nestor-replay --device alpha --boot 1 --interface vcan0

# Custom server
nestor-replay --server http://localhost:8000
```

## TUI Flow

```
$ nestor-replay
Connecting to CyphalCloud...

Found 3 devices:
  [1] alpha  (last seen: 2026-03-10 14:32:05)
  [2] beta   (last seen: 2026-03-11 09:15:22)  
  [3] gamma  (last seen: 2026-03-12 16:45:00)

Select device [1-3]: 2

Found 5 boot sessions for 'beta':
  [1] Boot #42  (2026-03-10 14:32:05 - 14:45:00, 12,000 frames)
  [2] Boot #43  (2026-03-11 09:15:22 - 09:30:00, 8,500 frames)
  ...

Select boot(s) [1-5, or 'all']: 1,2

Replay options:
  Interface [vcan0]: 
  Speed [1.0x]: 2.0

Replaying 20,500 frames to vcan0 at 2.0x speed...
[0.000s] 123 [4] DE AD BE EF
[0.001s] 456 [8] 01 02 03 04 05 06 07 08
...
Done! Replayed 20,500 frames in 45.2s
```

## Dependencies

- `reqwest` - HTTP client
- `tokio` - async runtime
- `socketcan` - SocketCAN interface  
- `dialoguer` - interactive prompts
- `indicatif` - progress bars
- `clap` - CLI argument parsing
- `serde` / `serde_json` - JSON parsing

## Replay Logic

1. Fetch all records for selected boots (paginate if >10k)
2. Sort by `hw_ts_us` (hardware timestamp)
3. For each frame:
   - Calculate delay from previous frame: `(current.hw_ts_us - prev.hw_ts_us) / speed_multiplier`
   - Sleep for delay (in microseconds)
   - Send frame to SocketCAN
   - Print frame (candump-style output)

## SocketCAN Frame Construction

```rust
// Convert API frame to SocketCAN frame
let can_id = if frame.extended {
    frame.can_id | CAN_EFF_FLAG
} else {
    frame.can_id
};

let data = hex::decode(&frame.data_hex)?;
let frame = CanFrame::new(can_id, &data)?;
socket.write_frame(&frame)?;
```
