#!/usr/bin/env python3
"""Vendor licensed native code fonts and compare them with Electron's locked font.

No package scripts run. Downloads are bounded and verified before parsing. The
upstream TTFs remain unmodified and their complete OFL notice is retained.
"""
from __future__ import annotations
import argparse
import base64
import hashlib
import io
import json
from pathlib import Path
import tarfile
import urllib.request

REFERENCE_INTEGRITY='WBA9elru6Jdp5df2mES55wuOO0WIrn3kpXnI4+W2ek5u3ZgLS9XS4gmIlcQhiZOWEKl95meYdvK7xI+ETLCq/Q=='
SOURCE='https://raw.githubusercontent.com/JetBrains/JetBrainsMono/v2.304/'
FILES={
 'crates/synara-app/assets/fonts/JetBrainsMono-Variable.ttf':('fonts/variable/JetBrainsMono%5Bwght%5D.ttf','b60e77f5dbf5505436c1904cb7a9ac4111ee76d0'),
 'crates/synara-app/assets/fonts/JetBrainsMono-Italic-Variable.ttf':('fonts/variable/JetBrainsMono-Italic%5Bwght%5D.ttf','5414835536ecf67b336e82642dbba2c5d2f32dfb'),
 'crates/synara-app/assets/licenses/JetBrainsMono-OFL.txt':('OFL.txt','8bee4148c1d54dbf5dae6d6c117fc80414266abb'),
}

def blob(data):return hashlib.sha1(f'blob {len(data)}\0'.encode()+data).hexdigest()
def download(url,limit):
    with urllib.request.urlopen(url,timeout=45) as response:
        data=response.read(limit+1)
    if len(data)>limit:raise ValueError('Font input exceeds its reviewed size limit')
    return data

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prepare',action='store_true')
    parser.add_argument('--compare-reference',action='store_true')
    options=parser.parse_args()
    root=Path(__file__).resolve().parents[2]
    for name,(remote,expected) in FILES.items():
        path=root/name
        if options.prepare:
            data=download(SOURCE+remote,1024*1024)
            if blob(data)!=expected:raise ValueError(f'Pinned font input changed: {name}')
            path.parent.mkdir(parents=True,exist_ok=True)
            path.write_bytes(data)
        if path.is_symlink() or blob(path.read_bytes())!=expected:
            raise ValueError(f'Bundled font or retained license changed: {name}')
    if not options.compare_reference:
        print('Verified both unmodified native font files and their retained OFL license')
        return
    from fontTools.ttLib import TTFont
    from fontTools.pens.recordingPen import DecomposingRecordingPen
    archive=download('https://registry.npmjs.org/@fontsource-variable/jetbrains-mono/-/jetbrains-mono-5.2.8.tgz',8*1024*1024)
    if hashlib.sha512(archive).digest()!=base64.b64decode(REFERENCE_INTEGRITY):
        raise ValueError('Electron font package does not match its immutable bun.lock integrity')
    checks=[]
    with tarfile.open(fileobj=io.BytesIO(archive),mode='r:gz') as package:
        metadata=package.getmember('package/package.json')
        if not metadata.isfile() or metadata.size>32768:raise ValueError('Unsafe font metadata')
        package_json=json.load(package.extractfile(metadata))
        if package_json['version']!='5.2.8' or package_json['license']!='OFL-1.1':
            raise ValueError('Unexpected locked font version or license')
        for style,native in [('normal','JetBrainsMono-Variable.ttf'),('italic','JetBrainsMono-Italic-Variable.ttf')]:
            member=package.getmember(f'package/files/jetbrains-mono-latin-wght-{style}.woff2')
            if not member.isfile() or member.size>1024*1024:raise ValueError('Unsafe font member')
            reference=TTFont(io.BytesIO(package.extractfile(member).read()),recalcTimestamp=False)
            target=TTFont(root/'crates/synara-app/assets/fonts'/native,recalcTimestamp=False)
            if target['head'].unitsPerEm!=reference['head'].unitsPerEm:raise ValueError('Font unit scales differ')
            codepoints=sorted(reference.getBestCmap())
            if not set(codepoints).issubset(target.getBestCmap()):raise ValueError('Native font misses a reference glyph')
            for weight in [400,700]:
                left=reference.getGlyphSet(location={'wght':weight})
                right=target.getGlyphSet(location={'wght':weight})
                for point in codepoints:
                    left_glyph=left[reference.getBestCmap()[point]]
                    right_glyph=right[target.getBestCmap()[point]]
                    if abs(left_glyph.width-right_glyph.width)>0.001:
                        raise ValueError(f'{style} {weight}: glyph advance differs at U+{point:04X}')
                    a=DecomposingRecordingPen(left)
                    b=DecomposingRecordingPen(right)
                    left_glyph.draw(a)
                    right_glyph.draw(b)
                    if a.value!=b.value:
                        raise ValueError(f'{style} {weight}: glyph outline differs at U+{point:04X}')
                checks.append({'style':style,'weight':weight,'codepoints':len(codepoints),'native_font_glyphs':len(target.getBestCmap())})
    evidence={
        'reference_revision':'eaa61eded31b6755d4f30ba8eabc5d905cf817cb',
        'reference_package':'@fontsource-variable/jetbrains-mono@5.2.8',
        'reference_integrity':'sha512-'+REFERENCE_INTEGRITY,
        'native_release':'JetBrainsMono/v2.304',
        'checks':checks,
        'limits':'Glyph outlines and advance widths match at the checked weights. This does not equate platform font rasterizers or prove pixel identity.',
    }
    output=root/'docs/ui/jetbrains-mono-reference-check.json'
    output.write_text(json.dumps(evidence,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(evidence,indent=2))

if __name__=='__main__':main()
