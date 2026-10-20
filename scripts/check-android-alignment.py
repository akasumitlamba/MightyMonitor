#!/usr/bin/env python3
"""Check actual JNI ELF load-segment alignment for Android 16 KiB page support."""
import argparse
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('libraries', nargs='+', type=Path)
args = parser.parse_args()
for library in args.libraries:
    result = subprocess.run(['readelf', '-lW', str(library)], check=True, capture_output=True, text=True)
    loads = [line.split() for line in result.stdout.splitlines() if line.lstrip().startswith('LOAD ')]
    if not loads:
        raise SystemExit(f'No ELF load segments: {library.name}')
    for segment in loads:
        offset, address, alignment = int(segment[1], 0), int(segment[2], 0), int(segment[-1], 0)
        if alignment < 16384 or alignment & (alignment - 1) or offset % 16384 != address % 16384:
            raise SystemExit(f'JNI library does not support 16 KiB load alignment: {library.parent.name}/{library.name}')
    print(f'16 KiB ELF alignment passed: {library.parent.name}/{library.name}')
