# Rust/GPUI and Electron Synara benchmark

Recorded locally on 2026-09-19. This compares the current optimized native rewrite
with the installed Electron Synara 0.8.4 production package, as built before
subsequent license metadata changes. Both use isolated,
empty profiles on one private Xvfb display. The user's running app and profile
were left intact.

## Results

| Metric | Rust/GPUI release | Electron 0.8.4 |
| --- | ---: | ---: |
| First mapped window | 145 ms [135–156] | 535 ms [514–568] |
| Idle memory (PSS) | 149.6 MiB [149.5–149.9] | 651.9 MiB [648.1–655.1] |
| Private memory (USS) | 139.3 MiB [139.1–139.5] | 498.0 MiB [494.3–502.2] |
| Summed RSS | 212.8 MiB [212.7–212.9] | 1169.0 MiB [1163.3–1171.5] |
| Idle CPU (one core) | 0.30% [0.30–0.50] | 1.90% [1.90–2.00] |
| Process count | 1 [1–1] | 8 [8–8] |

Values are medians of five warm launches per app; brackets show the observed
minimum–maximum. Each memory result is the median of eleven samples from one
10-second idle interval, following 15 seconds of settling. Fresh-profile results
are recorded separately and excluded from these medians.

### Single fresh-profile launches

| Metric | Rust/GPUI | Electron |
| --- | ---: | ---: |
| First mapped window | 218 ms | 750 ms |
| Idle memory (PSS) | 172.9 MiB | 750.3 MiB |
| Private memory (USS) | 162.2 MiB | 591.4 MiB |
| Summed RSS | 236.4 MiB | 1293.7 MiB |
| Idle CPU (one core) | 0.30% | 1.90% |
| Process count | 1 | 8 |

## What was measured

- **Window mapping:** monotonic elapsed time from spawning the executable until
  X11 reports a viewable window at least 840 × 620 pixels. Polled every 10 ms.
  This measures window creation; it does not establish first meaningful paint,
  interactive readiness, model readiness or session restoration latency.
- **PSS:** proportional resident memory, summed across the app's complete process
  family. Shared pages are apportioned, making this the primary footprint figure.
- **USS:** private clean + private dirty pages across that family. **RSS:** total
  resident pages; summed RSS double-counts pages shared between app processes.
- **Idle CPU:** process-family user + system CPU ticks divided by measured wall
  time. 100% means one fully occupied logical core. This has 10 ms tick precision
  on this host. The sampler and Xvfb process are excluded from app totals.
- **Processes:** main process plus descendants/session members, with a unique
  inherited marker to recover owned orphan helpers. Chromium helpers whose
  environment is unreadable are still counted by ancestry/session. A PSS/USS
  aggregate becomes unavailable if any member cannot be read; partial sums are
  never presented as the whole app.

These are observed idle working sets, not allocator measurements, startup peak
memory, GPU VRAM or total operating-system cost.

## Candidates and environment

- Native: local `cmdr-chara/native-ui-parity`, baseline `3df6a174419b997ee2ef47cd03b25b9024ec9e9e` plus
  uncommitted UI/Studio/Settings changes. Release source fingerprint
  `a4b879908ba239ee9adf2c119a8924755889da1522e209e3c6c2bf15b465edd5`. Compiler: `rustc 1.98.1 (48a229cea 2026-09-01) (Arch Linux rust 1:1.98.1-1)`.
- Electron: version `0.8.4`, packaged commit
  `70f5ed0e4757c0f69891b258171da80d324f0e18`, tag `v0.8.4`.
  `Chrome/150.0.7871.224`, Electron 43.4.1 (reported by the packaged runtime).
- Exact binary, archive, source and harness hashes are in
  [provenance.json](provenance.json). The source manifest is the frozen
  [benchmark source manifest](benchmark-source-manifest.json). It records the
  source tree at measurement time, including a license file since removed from
  the current branch; it is not a current file inventory.

- CPU: 13th Gen Intel(R) Core(TM) i5-1334U; 12 logical cores.
- RAM: 31.0 GiB; kernel/platform: `Linux-7.2.5-1-cachyos-bore-x86_64-with-glibc2.44`.
- AC power connected. Governor `powersave`, driver
  `intel_pstate`, energy preference `performance`.
- Formal measurement window: 2026-09-19T19:49:36.131073+00:00 to 2026-09-19T19:54:51.113085+00:00.
- Observed host CPU during warm idle samples: Rust
  6.4–9.7%, Electron
  6.0–7.9% of all cores.
  The desktop and user test window continued running, so this is not a quiescent
  dedicated benchmark host.

### Shared workload and controls

1. Empty home screen: no chats, projects, terminal sessions or prompted agents.
   Electron's first-run tour and application/provider update checks are disabled;
   its normal provider discovery and packaged services remain active.
2. Separate task-owned HOME, XDG config/data/cache/runtime and `SYNARA_HOME`.
   No production conversations, credentials or settings are imported.
3. One private Xvfb server, 1600 × 1000, 24-bit color, 1× scale. Actual outer
   windows are normalized to 1420 × 930 before settling. Electron initially maps
   a 1428 × 934 wrapper; that initial map is the startup endpoint. The mouse is
   moved outside the window before sampling so no tooltip is active.
4. `LIBGL_ALWAYS_SOFTWARE=1`. Separate diagnostic launches confirmed **Mesa
   llvmpipe (LLVM 22.1.8)** for both: Rust uses wgpu's OpenGL adapter; Electron
   uses ANGLE/OpenGL and reports software compositing. These probes are excluded
   from timings; the measured Electron runs have no debugger attached. Renderer
   evidence is retained in `graphics/`.
5. One measured fresh-profile launch per app, then five warm launches each.
   Order: Electron fresh, Rust fresh, then R/E, E/R, R/E, E/R, R/E. The same app's
   profile/cache persists between warm launches. Every run exits cleanly before
   the next app starts.
6. Filesystem caches are **not flushed**. Fresh profile does not mean cold OS
   cache. Preliminary launches and hashing have already warmed executable data.
   No build or test suite runs concurrently with the formal measurements.
7. The native executable is a byte-identical copy at a separate inode so it
   does not share executable pages with the Rust window left open for the user.
   Ordinary shared system libraries and background desktop processes remain.
8. Electron runs from the extracted official AppImage using `--no-sandbox
   --ozone-platform=x11`, consistent with the local wrapper's sandbox option.
   AppImage extraction/mount overhead is excluded. Rust runs with
   `GPUI_PLATFORM=x11 GPUI_X11_SCALE_FACTOR=1`, release optimization and no fixture.

## Interpretation and limits

Under this empty-home, software-rendered workload, native median PSS was
**77.0% lower**. Median window-mapping time was **145 ms versus 535 ms**,
a **72.9% reduction** (Electron/native ratio: 3.69).
All twelve formal launches completed, all process-family memory samples were
readable, and both applications closed cleanly in every trial.

The Electron application has more completed features and services than the
native rewrite. This is a comparison of these two application builds, not proof
of a language-level advantage. The current Rust UI still has the unported
features listed in [the parity record](../../ui/parity-port.md).

Five runs on one active desktop are an initial baseline. Background load,
power/thermal state and shared-library residency can affect it. CPU figures near
zero are especially sensitive to short sampling windows. No speed claim is made
for real chat generation, large transcripts, animations, frame pacing, disk I/O,
hardware-accelerated Wayland or other operating systems. Settled captures confirm
the measured home screens; they do not certify complete product parity.

## Reproduce

Prerequisites: the same candidate builds, Python with Pillow, Xvfb, libX11 and
libXtst. Run from the repository root; choose a new, nonexistent output directory.
The harness imports this repository's `scripts/native_smoke.py` Desktop helper.

```sh
cargo build --locked --release -p synara-app --bin synara-app
mkdir -p /tmp/synara-benchmark-candidate
cp target/release/synara-app /tmp/synara-benchmark-candidate/synara-app
python3 docs/benchmarks/2026-09-19-rust-electron/benchmark.py \
  --repo "$PWD" \
  --rust /tmp/synara-benchmark-candidate/synara-app \
  --electron /absolute/path/to/extracted-AppImage/synara \
  --output /tmp/synara-comparison-new \
  --runs 5 --settle 15 --idle 10
```

Use an already extracted AppImage to reproduce the timing boundary. The package
used here has a SquashFS offset of 188392 bytes and was extracted with
`unsquashfs -o 188392 -d /tmp/synara-electron-extracted /path/to/Synara-0.8.4-x86_64.AppImage`.
Keep the installed Electron directory intact because the executable needs its
adjacent runtime/resources. Probe the renderer again if changing the host.

## Evidence

- [Raw samples](raw-results.json), including per-process readings, complete run
  order, timestamps, window sizes, exit codes and host CPU load.
- [Trial CSV](trials.csv) and [aggregate JSON](summary.json).
- [Benchmark harness](benchmark.py) and [candidate hashes](provenance.json).
- [Electron home](captures/electron-warm-1.png) and
  [Rust home](captures/rust-warm-1.png) after idle sampling.
- [Excluded preliminary runs](excluded-runs.json). The interrupted earlier
  formal attempt was excluded before reporting because the executable inode
  was shared with the visible user app. Early pilots corrected whole-process
  accounting and window detection; their numbers are not mixed into this report.

Only measurements, diagnostic metadata and empty-profile captures are copied
here. Generated profiles, local authentication keys and browser storage remain
outside the repository.
