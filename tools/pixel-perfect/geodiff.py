#!/usr/bin/env python3
"""Geometry-only diff: blur both captures heavily so text AA noise vanishes,
then measure. Layout divergences survive blurring; rasterizer noise doesn't.
Usage: geodiff.py ref.png test.png [radius]"""
import sys
from PIL import Image, ImageFilter
import numpy as np
ref = Image.open(sys.argv[1]).convert("L")
test = Image.open(sys.argv[2]).convert("L")
if test.size != ref.size:
    test = test.resize(ref.size, Image.LANCZOS)
r = float(sys.argv[3]) if len(sys.argv) > 3 else 3.0
a = np.asarray(ref.filter(ImageFilter.GaussianBlur(r)), dtype=np.int16)
b = np.asarray(test.filter(ImageFilter.GaussianBlur(r)), dtype=np.int16)
d = np.abs(a - b)
print(f"geodiff r{r}: mean {d.mean():.2f}/255, >10: {(d>10).mean()*100:.2f}%, >25: {(d>25).mean()*100:.2f}%")
