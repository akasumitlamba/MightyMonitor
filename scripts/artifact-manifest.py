#!/usr/bin/env python3
"""Record the source revision and SHA-256 of explicitly selected build artifacts."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('artifacts', nargs='+', type=Path)
    args = parser.parse_args()
    artifacts = []
    names = set()
    for path in args.artifacts:
        if not path.is_file() or path.name in names:
            parser.error('Artifacts must exist and have distinct filenames')
        names.add(path.name)
        try:
            relative = path.resolve().relative_to(args.output.resolve()).as_posix()
        except ValueError:
            parser.error('Artifacts must be inside the output directory')
        if any(character in relative for character in ('\n', '\r', '\\')):
            parser.error('Artifact paths must be safe checksum filenames')
        digest = hashlib.sha256()
        with path.open('rb') as source:
            for block in iter(lambda: source.read(1024 * 1024), b''):
                digest.update(block)
        artifacts.append({'file': relative, 'bytes': path.stat().st_size, 'sha256': digest.hexdigest()})
    artifacts.sort(key=lambda item: item['file'])
    root = Path(__file__).resolve().parent.parent
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    dirty = bool(subprocess.check_output(['git', 'status', '--porcelain', '--untracked-files=no'], cwd=root, text=True).strip())
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / 'SHA256SUMS').write_text(''.join(f"{item['sha256']}  {item['file']}\n" for item in artifacts), encoding='utf-8', newline='\n')
    (args.output / 'BUILD_INFO.json').write_text(json.dumps({
        'project': 'MightyMonitor', 'revision': revision, 'tracked_sources_modified': dirty,
        'artifacts': artifacts,
    }, indent=2) + '\n', encoding='utf-8', newline='\n')


if __name__ == '__main__':
    main()
