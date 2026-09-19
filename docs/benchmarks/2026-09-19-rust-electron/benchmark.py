#!/usr/bin/env python3
"""Owned X11 empty-home benchmark. Does not read either app's real profile."""
import argparse
import ctypes as C
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import signal
import statistics
import subprocess
import sys
import time


class Attributes(C.Structure):
    _fields_ = [(n, t) for n, t in [
        ('x', C.c_int), ('y', C.c_int), ('width', C.c_int), ('height', C.c_int),
        ('border_width', C.c_int), ('depth', C.c_int), ('visual', C.c_void_p),
        ('root', C.c_ulong), ('class_', C.c_int), ('bit_gravity', C.c_int),
        ('win_gravity', C.c_int), ('backing_store', C.c_int),
        ('backing_planes', C.c_ulong), ('backing_pixel', C.c_ulong),
        ('save_under', C.c_int), ('colormap', C.c_ulong), ('map_installed', C.c_int),
        ('map_state', C.c_int), ('all_event_masks', C.c_long), ('your_event_mask', C.c_long),
        ('do_not_propagate_mask', C.c_long), ('override_redirect', C.c_int), ('screen', C.c_void_p)]]


def mapped_window(ui, parent_window=None, depth=0):
    root, parent, children, count = C.c_ulong(), C.c_ulong(), C.POINTER(C.c_ulong)(), C.c_uint()
    ui.x.XQueryTree(ui.display, parent_window or ui.root, C.byref(root), C.byref(parent), C.byref(children), C.byref(count))
    try:
        for window in list(children[:count.value]):
            attr = Attributes()
            if not ui.x.XGetWindowAttributes(ui.display, window, C.byref(attr)) or attr.map_state != 2:
                continue
            if attr.width >= 840 and attr.height >= 620:
                ui.window = window
                return window
    finally:
        if children:
            ui.x.XFree(children)
    return None


def processes(marker, root_pid):
    found, stats = {}, {}
    needle = f'SYNARA_BENCHMARK_RUN={marker}'.encode()
    for entry in Path('/proc').iterdir():
        if not entry.name.isdecimal():
            continue
        try:
            raw = (entry / 'stat').read_text()
            fields = raw[raw.rindex(')') + 2:].split()
            stats[int(entry.name)] = fields
        except (FileNotFoundError, ProcessLookupError, PermissionError):
            continue
    owned = {root_pid}
    # Use process ancestry/session, not inherited environment alone: Chromium
    # makes some helper environments inaccessible to ordinary /proc readers.
    while True:
        expanded = owned | {pid for pid, fields in stats.items() if int(fields[1]) in owned or int(fields[3]) == root_pid}
        if expanded == owned:
            break
        owned = expanded
    for pid in stats:
        if pid in owned:
            continue
        try:
            if needle in Path(f'/proc/{pid}/environ').read_bytes().split(b'\0'):
                owned.add(pid)
        except (FileNotFoundError, ProcessLookupError, PermissionError):
            continue
    for pid in owned:
        entry = Path(f'/proc/{pid}')
        try:
            fields = stats.get(pid)
            if fields is None:
                continue
            if fields[0] == 'Z':
                continue
            memory = {}
            try:
                for line in (entry / 'smaps_rollup').read_text().splitlines():
                    parts = line.split()
                    if len(parts) == 3 and parts[2] == 'kB':
                        memory[parts[0].rstrip(':')] = int(parts[1])
            except PermissionError:
                pass
            found[pid] = {
                'name': (entry / 'comm').read_text().strip(),
                'ticks': int(fields[11]) + int(fields[12]),
                'rss_kib': int(fields[21]) * os.sysconf('SC_PAGE_SIZE') // 1024,
                'pss_kib': memory.get('Pss'),
                'uss_kib': memory['Private_Clean'] + memory['Private_Dirty'] if 'Private_Clean' in memory else None,
            }
        except (FileNotFoundError, ProcessLookupError):
            continue
    return found


def host_cpu():
    fields = [int(v) for v in Path('/proc/stat').read_text().splitlines()[0].split()[1:9]]
    return sum(fields), fields[3] + fields[4]


def prepare(root, kind):
    root.mkdir(parents=True)
    for name in ['home', 'config', 'cache', 'runtime', 'data', 'synara/userdata']:
        (root / name).mkdir(parents=True, mode=0o700)
    if kind == 'electron':
        (root / 'synara/userdata/desktop-window-state.json').write_text(json.dumps({
            'version': 1, 'bounds': {'x': 0, 'y': 0, 'width': 1420, 'height': 930}, 'isMaximized': False}))
        # Bypass only first-run onboarding and update checks, leaving normal
        # rendering/provider discovery in place. No chats, credentials or projects.
        (root / 'synara/userdata/settings.json').write_text(json.dumps({
            'onboardingCompletedAt': '2026-09-19T00:00:00.000Z', 'enableProviderUpdateChecks': False}))


def trial(ui, binary, root, kind, label, output, settle, duration):
    marker = str(output / label)
    env = {'PATH': '/usr/bin:/bin', 'HOME': str(root / 'home'),
           'XDG_CONFIG_HOME': str(root / 'config'), 'XDG_CACHE_HOME': str(root / 'cache'),
           'XDG_DATA_HOME': str(root / 'data'), 'XDG_RUNTIME_DIR': str(root / 'runtime'),
           'SYNARA_HOME': str(root / 'synara'), 'SYNARA_DISABLE_AUTO_UPDATE': '1',
           'DISPLAY': ui.name, 'LIBGL_ALWAYS_SOFTWARE': '1', 'SYNARA_BENCHMARK_RUN': marker}
    args = [str(binary)]
    if kind == 'rust':
        args += ['--data-dir', str(root / 'data/native')]
        env.update(GPUI_PLATFORM='x11', GPUI_X11_SCALE_FACTOR='1', RUST_LOG='warn')
    else:
        args += ['--no-sandbox', '--ozone-platform=x11']
    ui.window = None
    record = {'kind': kind, 'label': label, 'command': args, 'status': 'failed',
              'started_at_utc': datetime.now(timezone.utc).isoformat()}
    with (output / f'{label}.log').open('w') as log:
        start = time.perf_counter()
        process = subprocess.Popen(args, env=env, cwd=root, stdin=subprocess.DEVNULL,
                                   stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            while not mapped_window(ui):
                if process.poll() is not None:
                    raise RuntimeError(f'{kind} exited {process.returncode} before mapping its main window')
                if time.perf_counter() - start > 45:
                    raise RuntimeError(f'{kind} did not map a usable window within 45 seconds')
                time.sleep(0.01)
            record['first_mapped_window_ms'] = (time.perf_counter() - start) * 1000
            record['initial_geometry'] = list(ui.geometry())
            # Chromium's X11 wrapper adds a few pixels around its requested
            # client area. Normalize the actual mapped outer window for idle.
            operation = ui.x.XMoveResizeWindow
            operation.argtypes = [C.c_void_p, C.c_ulong, C.c_int, C.c_int, C.c_uint, C.c_uint]
            operation(ui.display, ui.window, 0, 0, 1420, 930)
            ui.x.XFlush(ui.display)
            time.sleep(0.1)
            record['idle_geometry'] = list(ui.geometry())
            if record['idle_geometry'][2:] != [1420, 930]:
                raise RuntimeError(f'Non-comparable idle window size: {record["idle_geometry"]}')
            ui.focus()
            ui.xt.XTestFakeMotionEvent(ui.display, -1, 1500, 950, 0)
            ui.x.XFlush(ui.display)
            time.sleep(settle)
            cpu_ticks = {}
            samples = []
            start_host = host_cpu()
            idle_start = time.perf_counter()
            first_ticks = 0
            for index in range(duration + 1):
                if process.poll() is not None:
                    raise RuntimeError('App exited while measuring idle resources')
                current = processes(marker, process.pid)
                if not current:
                    raise RuntimeError('No owned process memory samples')
                for pid, info in current.items():
                    cpu_ticks[pid] = info['ticks']
                if index == 0:
                    first_ticks = sum(cpu_ticks.values())
                samples.append({'elapsed_s': time.perf_counter() - idle_start, 'processes': current,
                                **{field: sum(info[field] for info in current.values()) if all(info[field] is not None for info in current.values()) else None for field in ['rss_kib', 'pss_kib', 'uss_kib']}})
                if index < duration:
                    time.sleep(max(0, idle_start + index + 1 - time.perf_counter()))
            elapsed = time.perf_counter() - idle_start
            end_host = host_cpu()
            record.update(status='passed', samples=samples, idle_seconds=elapsed,
                          idle_cpu_one_core_percent=(sum(cpu_ticks.values()) - first_ticks) / os.sysconf('SC_CLK_TCK') / elapsed * 100,
                          host_cpu_all_cores_percent=(1 - (end_host[1] - start_host[1]) / (end_host[0] - start_host[0])) * 100,
                          process_count_median=statistics.median(len(s['processes']) for s in samples))
            for field in ['rss_kib', 'pss_kib', 'uss_kib']:
                values = [s[field] for s in samples]
                record[f'{field}_median'] = statistics.median(values) if all(v is not None for v in values) else None
                record[f'{field}_observed_peak'] = max(values) if all(v is not None for v in values) else None
            if label.endswith('fresh') or label.endswith('warm-1'):
                ui.screenshot(label, window_only=True)
            ui.request_close()
            process.wait(timeout=15)
            record['exit_code'] = process.returncode
            if process.returncode != 0:
                record['status'] = 'failed'
                record['error'] = 'Nonzero exit after graceful window close'
        except Exception as error:
            record['error'] = str(error)
            ui.screenshot(label + '-failure', window_only=bool(ui.window))
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=5)
            # Reap only owned helpers that inherited this trial's unique marker.
            for pid in processes(marker, process.pid):
                try:
                    os.kill(pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
    (output / f'{label}.json').write_text(json.dumps(record, indent=2) + '\n')
    print(json.dumps({k: v for k, v in record.items() if k not in ('samples', 'command')}), flush=True)
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--rust', type=Path, required=True)
    parser.add_argument('--electron', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--runs', type=int, default=5)
    parser.add_argument('--settle', type=int, default=15)
    parser.add_argument('--idle', type=int, default=10)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True)
    sys.path.insert(0, str(args.repo.resolve() / 'scripts'))
    from native_smoke import Desktop
    ui = Desktop(output)
    ui.x.XGetWindowAttributes.argtypes = [C.c_void_p, C.c_ulong, C.POINTER(Attributes)]
    ui.x.XGetWindowAttributes.restype = C.c_int
    binaries = {'rust': args.rust.resolve(), 'electron': args.electron.resolve()}
    report = {'started_at_utc': datetime.now(timezone.utc).isoformat(),
              'platform': platform.platform(), 'cpu_count': os.cpu_count(),
              'cpuinfo': next(line.split(':', 1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines() if line.startswith('model name')),
              'memory': Path('/proc/meminfo').read_text().splitlines()[0],
              'conditions': {'graphics': 'private Xvfb/X11; LIBGL_ALWAYS_SOFTWARE=1; separate probes verified Rust wgpu GL llvmpipe and Electron ANGLE llvmpipe with software compositing', 'window': [1420, 930],
                             'scale': 1, 'settle_after_map_s': args.settle, 'idle_sample_s': args.idle,
                             'os_cache': 'not dropped; fresh profile is not cold filesystem cache',
                             'warmups': 'one measured fresh-profile run per app, excluded from warm medians',
                             'order': 'alternating pairs', 'workload': 'empty home, no chats/projects/agents; Electron onboarding/update checks disabled'},
              'binary_sha256': {k: hashlib.sha256(v.read_bytes()).hexdigest() for k, v in binaries.items()}, 'runs': []}
    try:
        for kind in ['electron', 'rust']:
            prepare(output / kind, kind)
            row = trial(ui, binaries[kind], output / kind, kind, kind + '-fresh', output, args.settle, args.idle)
            report['runs'].append(row)
            if row['status'] != 'passed':
                raise RuntimeError(row.get('error', 'fresh-profile run failed'))
        for i in range(1, args.runs + 1):
            for kind in (['rust', 'electron'] if i % 2 else ['electron', 'rust']):
                row = trial(ui, binaries[kind], output / kind, kind, f'{kind}-warm-{i}', output, args.settle, args.idle)
                report['runs'].append(row)
                if row['status'] != 'passed':
                    raise RuntimeError(row.get('error', 'warm run failed'))
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        ui.close()
        report['finished_at_utc'] = datetime.now(timezone.utc).isoformat()
        (output / 'raw-results.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
