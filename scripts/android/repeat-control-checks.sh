#!/usr/bin/env bash
# Stress the previously intermittent paths; a failed repetition is never retried.
set -euo pipefail
case "${1:-}" in 29|36) ;; *) exit 0 ;; esac
emulator_serial=$(adb get-serialno)
case "$emulator_serial" in emulator-*) ;; *) echo 'Control checks require an isolated Android emulator'; exit 2 ;; esac
trap 'adb -s "$emulator_serial" logcat -d > /tmp/mm-android-repeat-logcat.log 2>/dev/null || true' EXIT
for iteration in 1 2; do
    echo "Independent control check $iteration"
    output="/tmp/mm-control-repeat-$iteration.log"
    adb -s "$emulator_serial" shell am instrument -e remoteControlTest true -e class com.mightymonitor.app.ViewerSmokeTest\#controlsPhoneOnlyAfterBothGrantsAndStopsOnRevocation -w com.mightymonitor.app.test/androidx.test.runner.AndroidJUnitRunner | tee "$output"
    grep -q 'OK (1 test)' "$output"
done
