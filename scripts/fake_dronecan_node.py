#!/usr/bin/env python3
"""
Fake DroneCAN node that publishes NodeStatus at ~1 Hz.
Useful for testing dronecan_live_plot.py.

Usage:
  python3 scripts/fake_dronecan_node.py -i vcan0
  python3 scripts/fake_dronecan_node.py -i vcan0 --node-id 42 --rate 5
"""

from __future__ import annotations

import argparse
import math
import time
import sys

try:
    import dronecan
    # python-can's socketcan backend doesn't implement flush_tx_buffer,
    # but dronecan's writer thread calls it. Patch to a no-op.
    from can.interfaces.socketcan import SocketcanBus as _Bus
    _Bus.flush_tx_buffer = lambda self: None
except ImportError:
    print("pip install dronecan", file=sys.stderr)
    sys.exit(1)


def main():
    p = argparse.ArgumentParser(description="Fake DroneCAN node for testing.")
    p.add_argument("--interface", "-i", default="vcan0")
    p.add_argument("--node-id", "-n", type=int, default=10)
    p.add_argument("--rate", "-r", type=float, default=1.0, help="Messages per second.")
    args = p.parse_args()

    node = dronecan.make_node(args.interface, node_id=args.node_id, bitrate=1000000)
    t0 = time.monotonic()
    print(f"Publishing NodeStatus on {args.interface} (node {args.node_id}) at {args.rate} Hz. Ctrl+C to stop.")

    period = 1.0 / args.rate

    try:
        while True:
            elapsed = time.monotonic() - t0

            msg = dronecan.uavcan.protocol.NodeStatus()
            msg.uptime_sec = int(elapsed)
            msg.health = dronecan.uavcan.protocol.NodeStatus().HEALTH_OK
            msg.mode = dronecan.uavcan.protocol.NodeStatus().MODE_OPERATIONAL
            # Vary sub_mode with a sine wave so the plot is interesting
            msg.sub_mode = int(abs(math.sin(elapsed * 0.5)) * 15)
            msg.vendor_specific_status_code = int(50 + 30 * math.sin(elapsed * 0.3))

            node.broadcast(msg)

            # Spin briefly to process protocol, then sleep for rate control
            deadline = time.monotonic() + period
            while time.monotonic() < deadline:
                node.spin(timeout=0.01)
                remaining = deadline - time.monotonic()
                if remaining > 0.02:
                    time.sleep(0.01)
    except KeyboardInterrupt:
        pass
    finally:
        node.close()


if __name__ == "__main__":
    main()
