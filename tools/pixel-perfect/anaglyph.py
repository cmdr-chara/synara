#!/usr/bin/env python3
"""Anaglyph overlay diff: reference in red, test in cyan.
Matching pixels read grey/white; differences fringe red or cyan.
Usage: anaglyph.py ref.png test.png out.png"""
import sys
from PIL import Image
import numpy as np

ref = Image.open(sys.argv[1]).convert("L")
test_img = Image.open(sys.argv[2]).convert("L")
if test_img.size != ref.size:
    test_img = test_img.resize(ref.size, Image.LANCZOS)
r = np.asarray(ref)
c = np.asarray(test_img)
out = np.stack([r, c, c], axis=2)
Image.fromarray(out).save(sys.argv[3])
print("ok")
