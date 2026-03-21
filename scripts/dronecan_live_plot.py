#!/usr/bin/env python3
"""
dronecan_live_plot.py

Receive DroneCAN messages on a SocketCAN interface and plot selected
numeric fields in real time using matplotlib.

Requirements:
  pip install dronecan matplotlib

Setup (vcan0 for testing):
  sudo modprobe vcan
  sudo ip link add dev vcan0 type vcan
  sudo ip link set up vcan0

Usage:
  # Plot all numeric fields from NodeStatus
  python3 scripts/dronecan_live_plot.py -i vcan0

  # Plot ESC status, specific fields only
  python3 scripts/dronecan_live_plot.py -i vcan0 -m uavcan.equipment.esc.Status --fields voltage current

  # Increase history window to 120 seconds
  python3 scripts/dronecan_live_plot.py -i vcan0 -m uavcan.protocol.NodeStatus --window 120
"""

from __future__ import annotations

import argparse
import sys
import time
from collections import defaultdict
from typing import Optional

try:
    import dronecan
    # python-can's socketcan backend doesn't implement flush_tx_buffer,
    # but dronecan's writer thread calls it. Patch to a no-op.
    from can.interfaces.socketcan import SocketcanBus as _Bus
    _Bus.flush_tx_buffer = lambda self: None
except ImportError:
    print("ERROR: Missing 'dronecan'. Install via:", file=sys.stderr)
    print("  pip install dronecan", file=sys.stderr)
    sys.exit(1)

try:
    import matplotlib.pyplot as plt
    import matplotlib.animation as animation
except ImportError:
    print("ERROR: Missing 'matplotlib'. Install via:", file=sys.stderr)
    print("  pip install matplotlib", file=sys.stderr)
    sys.exit(1)


def parse_args(argv: Optional[list[str]] = None) -> argparse.Namespace:
    p = argparse.ArgumentParser(
        description="Receive DroneCAN messages and plot numeric fields in real time."
    )
    p.add_argument("--interface", "-i", default="vcan0",
                   help="SocketCAN interface (default: vcan0).")
    p.add_argument("--message", "-m", default="uavcan.protocol.NodeStatus",
                   help="DroneCAN message type (default: uavcan.protocol.NodeStatus).")
    p.add_argument("--fields", "-f", nargs="*", default=None,
                   help="Specific fields to plot (default: all numeric fields).")
    p.add_argument("--window", "-w", type=float, default=60.0,
                   help="Rolling time window in seconds (default: 60).")
    p.add_argument("--node-id", "-n", type=int, default=None,
                   help="Filter by source node ID (default: accept all).")
    return p.parse_args(argv)


def extract_numeric_fields(msg, prefix: str = "") -> dict[str, float]:
    """Recursively extract numeric (int/float) fields from a DroneCAN message."""
    fields = {}
    for field_name, field_value in msg._fields.items():
        full_name = f"{prefix}{field_name}" if prefix else field_name
        raw = getattr(field_value, "value", field_value)
        if isinstance(raw, (int, float)):
            fields[full_name] = float(raw)
        elif hasattr(field_value, "_fields"):
            fields.update(extract_numeric_fields(field_value, prefix=f"{full_name}."))
    return fields


def main(argv: Optional[list[str]] = None) -> int:
    args = parse_args(argv)

    # Resolve message type
    try:
        parts = args.message.split(".")
        msg_type = dronecan
        for part in parts:
            msg_type = getattr(msg_type, part)
    except AttributeError:
        print(f"ERROR: Unknown message type '{args.message}'.", file=sys.stderr)
        return 1

    print(f"Listening for {args.message} on {args.interface} ...")

    # DroneCAN node
    node = dronecan.make_node(args.interface, node_id=126, bitrate=1000000)

    # Data storage
    data: dict[str, tuple[list[float], list[float]]] = defaultdict(lambda: ([], []))
    t0 = time.monotonic()
    discovered_fields: list[str] = []

    def on_message(event):
        ts = time.monotonic() - t0

        if args.node_id is not None:
            if event.transfer.source_node_id != args.node_id:
                return

        numeric = extract_numeric_fields(event.message)
        if not numeric:
            return

        if args.fields:
            numeric = {k: v for k, v in numeric.items() if k in args.fields}

        for name, value in numeric.items():
            if name not in discovered_fields:
                discovered_fields.append(name)
            ts_list, val_list = data[name]
            ts_list.append(ts)
            val_list.append(value)

    node.add_handler(msg_type, on_message)

    # Set up matplotlib
    fig, ax = plt.subplots(figsize=(12, 6))
    fig.canvas.manager.set_window_title(f"DroneCAN: {args.message}")
    lines: dict[str, object] = {}

    def update(_frame):
        try:
            node.spin(timeout=0.01)
        except Exception:
            pass

        now = time.monotonic() - t0
        cutoff = now - args.window

        for name in list(discovered_fields):
            ts_list, val_list = data[name]

            while ts_list and ts_list[0] < cutoff:
                ts_list.pop(0)
                val_list.pop(0)

            if name not in lines:
                line, = ax.plot([], [], label=name, marker=".", markersize=3, linewidth=1)
                lines[name] = line
                ax.legend(loc="upper left", fontsize=8)

            lines[name].set_data(ts_list, val_list)

        if any(data[n][0] for n in discovered_fields):
            ax.set_xlim(max(0, now - args.window), now + 1)
            all_vals = [v for n in discovered_fields for v in data[n][1]]
            if all_vals:
                ymin, ymax = min(all_vals), max(all_vals)
                margin = max(0.1, (ymax - ymin) * 0.1)
                ax.set_ylim(ymin - margin, ymax + margin)

        ax.set_xlabel("Time (s)")
        ax.set_ylabel("Value")
        ax.set_title(f"{args.message}  [{args.interface}]")
        ax.grid(True, alpha=0.3)

        return list(lines.values())

    fig._ani = animation.FuncAnimation(fig, update, interval=100, cache_frame_data=False)
    plt.tight_layout()

    try:
        plt.show()
    except KeyboardInterrupt:
        pass
    finally:
        node.close()

    return 0


if __name__ == "__main__":
    sys.exit(main())
