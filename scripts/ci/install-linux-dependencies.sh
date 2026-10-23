#!/usr/bin/env bash
# GitHub's ephemeral Ubuntu runners only. Never reconfigure a developer's PC.
set -euo pipefail
if [[ ${GITHUB_ACTIONS:-} != true ]] || [[ $(dpkg --print-architecture) != amd64 ]]; then
  echo 'This dependency installer is only for GitHub Ubuntu amd64 runners.' >&2
  exit 2
fi
. /etc/os-release
[[ $ID == ubuntu ]] || exit 2
# The Azure HTTP mirror has repeatedly stalled individual package bodies. Use
# Ubuntu's official HTTPS archive while retaining distribution/signing settings.
sudo python3 - <<'PY'
from pathlib import Path
paths = [Path('/etc/apt/sources.list')]
paths += list(Path('/etc/apt/sources.list.d').glob('*.sources'))
paths += list(Path('/etc/apt/sources.list.d').glob('*.list'))
for path in paths:
    if not path.is_file():
        continue
    original = path.read_text()
    revised = original.replace('mirror+file:/etc/apt/apt-mirrors.txt', 'https://archive.ubuntu.com/ubuntu/')
    revised = revised.replace('http://azure.archive.ubuntu.com/ubuntu', 'https://archive.ubuntu.com/ubuntu')
    if original != revised:
        path.write_text(revised)
PY
options=(-o Acquire::Retries=2 -o Acquire::ForceIPv4=true -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30)
sudo apt-get "${options[@]}" update
sudo apt-get "${options[@]}" install --no-install-recommends -y -- "$@"
