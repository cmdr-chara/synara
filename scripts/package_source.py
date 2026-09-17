#!/usr/bin/env python3
"""Prepare a checksummed UTF-8 source checkpoint for the branch-only CI publisher."""
from pathlib import Path
import argparse
import base64
import gzip
import hashlib
import json
import re
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--base', required=True)
parser.add_argument('--message', required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
if not re.fullmatch(r'[0-9a-f]{40}', args.base):
    parser.error('base must be a full Git commit ID')
root = Path(__file__).resolve().parents[1]
paths = subprocess.check_output(['git', 'ls-files', '-z'], cwd=root).decode().split('\0')
files = {}
for path in filter(None, paths):
    if path.startswith('.github/workflows/') or path == '.synara-source.json':
        continue
    source = root / path
    if source.is_symlink():
        raise SystemExit(f'Symlinks cannot be source-packaged: {path}')
    files[path] = source.read_text()
data = json.dumps(files, sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()
package = dict(base_commit=args.base, message=args.message,
               sha256=hashlib.sha256(data).hexdigest(),
               gzip_base64=base64.b64encode(gzip.compress(data, mtime=0)).decode())
args.output.write_text(json.dumps(package, separators=(',', ':')) + '\n')
print(f'{len(files)} files, {len(data)} source bytes, {args.output.stat().st_size} transport bytes')
