# Portable device/simulator backend checkpoint

Roadmap owner: L1-L3 and M4.

This checkpoint defines the Rust-owned device/simulator domain and helper protocol.
It does **not** claim Apple simulator or physical-device support.

## Domain lifecycle

`synara-runtime::DeviceDescriptor` records a validated opaque device ID, display
name, platform, simulator/physical kind and explicit lifecycle state. Discovery is
bounded to 256 unique devices. `DeviceSessionState` owns attach/disconnect state,
display dimensions and monotonically increasing frame sequence acceptance.

Stale/duplicate frames, zero/oversized dimensions and frames whose byte count does
not match their pixel dimensions fail closed.

## Bounded helper IPC

The helper boundary is a small framed protocol:

- one byte packet kind;
- four-byte big-endian payload length;
- exact payload bytes;
- control/input payloads capped at 64 KiB;
- raw frame payloads capped at 32 MiB.

Control messages use typed serde structures for discover, attach, resize and
detach. Frame metadata carries session ID, dimensions, pixel format, byte length
and sequence. Raw frame bytes are a separate packet, avoiding a general
JSON/command bridge and avoiding base64 inflation.

Device input supports bounded tap, swipe, text and key operations. Creating an
input packet requires a `DeviceInputConsent` value, which is deliberately not
serializable. Agent/device data therefore cannot deserialize itself into the
application's user-input consent token.

## Helper ownership

`DeviceHelperOwner` owns a shared `ProcessHandle`. Drop, explicit stop and
shutdown all request process cleanup through the existing runtime supervisor,
independently of a GPUI view remaining alive. Lifecycle observation is exposed
through a watch channel.

This is the portable ownership contract for a future Apple helper. It does not
claim that a helper compiled against Apple APIs exists in this Linux repository
checkpoint.

## Evidence and blocker

Deterministic tests cover discovery limits/duplicate IDs, exact packet framing,
payload bounds, consent-gated input encoding, frame dimension/byte invariants,
monotonic frame sequencing, resize and post-disconnect rejection.

Remaining Apple-specific acceptance requires an actual supported macOS/iOS
environment and Apple SDK APIs to implement and exercise simulator/device
discovery, capture and input. L4 therefore remains externally blocked. L2/L3
remain open for that native helper interaction even though the portable protocol
and lifecycle side is implemented.


## September 21 continuation: Device adapters and viewer

The portable contracts above remain historical evidence. The current session adds
real command-backed device/notification adapters, a native screenshot viewer and
functional Settings controls. See [source inventory](../ui/device-settings.md) and
[validation receipt](device-settings-session.md) for exact scope and unsupported
targets. The native Apple helper protocol, OS credential-store adapter and hardware/
platform acceptance are not marked complete by these source additions.
