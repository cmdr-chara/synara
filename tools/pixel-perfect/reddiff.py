#!/usr/bin/env python3
"""Pixel-perfect overlay diff: two screenshots aligned, differences in red.
Usage: reddiff.py ref.png test.png out.png [threshold]
Output: test image with red where pixels differ from ref beyond threshold,
plus a diff-percentage report per region band."""
import sys
from PIL import Image, ImageChops
import numpy as np

ref_p, test_p, out_p = sys.argv[1], sys.argv[2], sys.argv[3]
thresh = int(sys.argv[4]) if len(sys.argv) > 4 else 18

ref = np.asarray(Image.open(ref_p).convert("RGB"), dtype=np.int16)
test_img = Image.open(test_p).convert("RGB")
if test_img.size != (ref.shape[1], ref.shape[0]):
    test_img = test_img.resize((ref.shape[1], ref.shape[0]), Image.LANCZOS)
test = np.asarray(test_img, dtype=np.int16)

diff = np.abs(ref - test).sum(axis=2)
mask = diff > thresh

out = np.asarray(test_img).copy()
# red tint where different, keep texture visible
out[mask] = (out[mask] * 0.25 + np.array([255, 0, 0]) * 0.75).astype(np.uint8)
Image.fromarray(out).save(out_p)

h = ref.shape[0]
bands = 6
print(f"total diff: {mask.mean()*100:.2f}% pixels (thresh {thresh})")
for i in range(bands):
    band = mask[i*h//bands:(i+1)*h//bands]
    print(f"  band {i} (y {i*h//bands}-{(i+1)*h//bands}): {band.mean()*100:.2f}%")
