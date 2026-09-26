#!/usr/bin/env python3
"""Regenerate native theme data from the immutable, MIT-licensed Electron source.

The catalog is parsed as a deliberately small object-literal grammar, never
executed. Optional reference vectors execute only the listed pure color helpers
from the hash-pinned source in Node's VM without process, require or filesystem
bindings. No network request, package installation or production profile access.
"""
from __future__ import annotations

import argparse
import ast
import hashlib
import json
from pathlib import Path
import re
import subprocess

REVISION = 'eaa61eded31b6755d4f30ba8eabc5d905cf817cb'
CATALOG_SHA = 'a4073e16cb41da8c41831924c454cd7e1f08fcff'
LOGIC_SHA = '667e88f09bc0edb139fe9933ad38aac9e911a710'
NAMES = {
    'absolutely', 'ayu', 'catppuccin', 'codex', 'synara', 'dracula', 'everforest',
    'github', 'gruvbox', 'linear', 'lobster', 'material', 'matrix', 'monokai',
    'night-owl', 'nord', 'notion', 'one', 'oscurange', 'proof', 'raycast',
    'rose-pine', 'sentry', 'solarized', 'temple', 'tokyo-night', 'vercel', 'vscode-plus',
}


def read_pinned(path: Path, expected: str) -> str:
    data = path.read_bytes()
    digest = hashlib.sha1(f'blob {len(data)}\0'.encode() + data).hexdigest()
    if digest != expected:
        raise ValueError(f'Reference source changed: {path}. Review and repin explicitly.')
    return data.decode('utf-8')


class LiteralParser:
    token = re.compile(r'''\s+|//[^\n]*|"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|[A-Za-z_$][\w$]*|-?(?:0|[1-9]\d*)(?:\.\d+)?|[{}:,;]''')

    def __init__(self, source: str):
        self.tokens = []
        cursor = 0
        for match in self.token.finditer(source):
            if match.start() != cursor:
                raise ValueError('Reference catalog contains non-literal TypeScript syntax')
            cursor = match.end()
            value = match.group()
            if not value.isspace() and not value.startswith('//'):
                self.tokens.append(value)
        if cursor != len(source):
            raise ValueError('Unparsed catalog data')
        self.index = 0

    def take(self, expected=None):
        if self.index >= len(self.tokens):
            raise ValueError('Truncated catalog')
        value = self.tokens[self.index]
        self.index += 1
        if expected is not None and value != expected:
            raise ValueError(f'Expected {expected}, received {value}')
        return value

    def value(self, depth=0):
        if depth > 12:
            raise ValueError('Catalog exceeds the permitted nesting depth')
        value = self.take()
        if value == '{':
            result = {}
            if self.tokens[self.index] == '}':
                self.take('}')
                return result
            while True:
                key = self.take()
                if key.startswith(('"', "'")):
                    key = ast.literal_eval(key)
                if not isinstance(key, str) or key in result:
                    raise ValueError('Invalid or duplicate catalog key')
                self.take(':')
                result[key] = self.value(depth + 1)
                delimiter = self.take()
                if delimiter == '}':
                    return result
                if delimiter != ',':
                    raise ValueError('Invalid object delimiter')
                if self.tokens[self.index] == '}':
                    self.take('}')
                    return result
        if value.startswith(('"', "'")):
            return ast.literal_eval(value)
        if value in ('true', 'false', 'null') or re.fullmatch(r'-?\d+(?:\.\d+)?', value):
            return json.loads(value)
        raise ValueError(f'Catalog expressions are not permitted: {value}')


def catalog_from_source(source: str):
    marker = 'export const THEME_SEED_CATALOG:'
    if source.count(marker) != 1:
        raise ValueError('Ambiguous theme catalog declaration')
    declaration = source[source.index(marker):]
    parser = LiteralParser(declaration[declaration.index('=') + 1:])
    catalog = parser.value()
    parser.take(';')
    if parser.index != len(parser.tokens) or set(catalog) != NAMES:
        raise ValueError('Catalog names or trailing source do not match the pinned contract')
    for name, variants in catalog.items():
        if not variants or not set(variants) <= {'light', 'dark'}:
            raise ValueError(f'Invalid variants for {name}')
        for theme in variants.values():
            if set(theme) != {'accent', 'contrast', 'fonts', 'ink', 'opaqueWindows', 'semanticColors', 'surface'}:
                raise ValueError('Unexpected theme fields')
            if type(theme['contrast']) is not int or not 0 <= theme['contrast'] <= 100 or type(theme['opaqueWindows']) is not bool:
                raise ValueError('Invalid theme scalar')
            if set(theme['fonts']) != {'ui', 'code'} or set(theme['semanticColors']) != {'diffAdded', 'diffRemoved', 'skill'}:
                raise ValueError('Invalid theme font or semantic-color shape')
            for font in theme['fonts'].values():
                if font is not None and (not isinstance(font, str) or not font or len(font.encode()) > 256 or any(ord(char) < 32 for char in font)):
                    raise ValueError('Invalid theme font name')
            for color in [theme['accent'], theme['ink'], theme['surface'], *theme['semanticColors'].values()]:
                if not isinstance(color, str) or not re.fullmatch(r'#[0-9a-fA-F]{6}', color):
                    raise ValueError('Invalid theme color')
    return catalog


def extract_pure_function(source: str, name: str, arguments: str) -> str:
    marker = 'function ' + name + '('
    if source.count(marker) != 1:
        raise ValueError(f'Ambiguous reference function {name}')
    start = source.index('{', source.index(marker))
    depth = 0
    quote = None
    escaped = False
    line_comment = False
    block_comment = False
    index = start
    while index < len(source):
        char = source[index]
        pair = source[index:index + 2]
        if line_comment:
            if char == '\n': line_comment = False
        elif block_comment:
            if pair == '*/': block_comment = False; index += 1
        elif quote:
            if escaped: escaped = False
            elif char == '\\': escaped = True
            elif char == quote: quote = None
        elif pair == '//': line_comment = True; index += 1
        elif pair == '/*': block_comment = True; index += 1
        elif char in ('"', "'", '`'): quote = char
        elif char == '{': depth += 1
        elif char == '}':
            depth -= 1
            if depth == 0:
                return f'function {name}({arguments}) ' + source[start:index + 1]
        index += 1
    raise ValueError(f'Unterminated reference function {name}')


def reference_vectors(logic: str, catalog):
    names = {
        'buildComputedTheme': 'theme, variant',
        'buildLightDerivedTokens': 'theme', 'buildDarkDerivedTokens': 'theme',
        'buildSurfaceUnder': 'theme, surface, ink, variant',
        'buildPanelBackground': 'theme',
        'buildComposerFocusBorder': 'pack, variant, panelBackground',
        'normalizeContrastStrength': 'value, variant',
        'parseHexColor': 'value', 'mixHex': 'from, to, amount',
        'mixRgb': 'from, to, amount', 'mixChannel': 'from, to, amount',
        'formatHex': 'color', 'formatOpaqueRgb': 'color', 'formatRgba': 'color, opacity',
        'formatHexChannel': 'value', 'formatAlpha': 'value',
    }
    program = '''
const BLACK = {red:0, green:0, blue:0};
const WHITE = {red:255, green:255, blue:255};
const CONTRAST_CURVE_BASELINE = {dark:60, light:45};
const CONTRAST_CURVE_BELOW_BASELINE = 0.7;
const CONTRAST_CURVE_ABOVE_BASELINE = 2;
const SURFACE_UNDER_BASE_ALPHA = {dark:0.16, light:0.04};
const SURFACE_UNDER_CONTRAST_STEP = {dark:0.0015, light:0.0012};
const PANEL_BASE_ALPHA = {dark:0.03, light:0.18};
const PANEL_CONTRAST_STEP = {dark:0.03, light:0.008};
'''
    program += '\n'.join(extract_pure_function(logic, name, arguments) for name, arguments in names.items())
    program += '''
const result = [];
for (const [id, variants] of Object.entries(catalog)) {
  for (const [variant, seed] of Object.entries(variants)) {
    for (const contrast of [0, 45, 60, 100]) {
      const theme = {...seed, contrast};
      const computed = buildComputedTheme(theme, variant);
      const derived = variant === 'dark' ? buildDarkDerivedTokens(computed) : buildLightDerivedTokens(computed);
      const panel = buildPanelBackground(computed);
      result.push({id, variant, contrast, strength:computed.contrast,
        surfaceUnder:computed.surfaceUnder, panel,
        editorBackground:formatOpaqueRgb(computed.editorBackground),
        composerFocusBorder:buildComposerFocusBorder({theme}, variant, panel), derived});
    }
  }
}
JSON.stringify(result);
'''
    launcher = "const vm=require('node:vm');const fs=require('node:fs');const input=JSON.parse(fs.readFileSync(0,'utf8'));const script=new vm.Script(input.program);const result=script.runInNewContext({catalog:input.catalog},{timeout:5000,contextCodeGeneration:{strings:false,wasm:false}});process.stdout.write(result);"
    run = subprocess.run(['node', '-e', launcher], input=json.dumps({'program': program, 'catalog': catalog}),
                         stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=True, timeout=15)
    cases = json.loads(run.stdout)
    def color(text):
        if re.fullmatch(r'#[0-9a-fA-F]{6}', text):
            return {'rgb': int(text[1:], 16), 'alpha': 1.0}
        match = re.fullmatch(r'rgba?\((\d+), (\d+), (\d+)(?:, ([0-9.]+))?\)', text)
        if not match:
            raise ValueError('Unexpected reference color format')
        r, g, b = map(int, match.group(1, 2, 3))
        return {'rgb': (r << 16) | (g << 8) | b, 'alpha': float(match.group(4) or 1)}
    for case in cases:
        case['derived'] = {key: color(value) for key, value in case['derived'].items()}
        for key in ('surfaceUnder', 'panel', 'editorBackground', 'composerFocusBorder'):
            case[key] = color(case[key])['rgb']
    return {'revision': REVISION, 'logic_blob': LOGIC_SHA, 'cases': cases}


def write_or_check(path: Path, value, check: bool):
    data = (json.dumps(value, sort_keys=True, ensure_ascii=False, indent=2) + '\n').encode()
    if check:
        if not path.is_file() or path.read_bytes() != data:
            raise SystemExit(f'Stale generated theme data: {path}')
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--vectors', action='store_true')
    options = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    reference = root / 'docs/ui/reference'
    catalog = catalog_from_source(read_pinned(reference / 'electron-theme.seed.ts', CATALOG_SHA))
    destination = root / 'crates/synara-workspace/src/settings/theme'
    write_or_check(destination / 'catalog.json', catalog, options.check)
    if options.vectors:
        vectors = reference_vectors(read_pinned(reference / 'electron-theme.logic.ts', LOGIC_SHA), catalog)
        write_or_check(destination / 'reference-vectors.json', vectors, options.check)
        print(f"Verified {len(vectors['cases'])} reference color cases")
    print(f'Verified {len(catalog)} immutable theme presets')


if __name__ == '__main__':
    main()
