#!/usr/bin/env python3
"""
Smoke test for pyroscope_cpython cdylib.

Loads the .so via ctypes, calls pyroscope_start(), then burns CPU
for a few seconds. In debug builds, the SIGPROF handler prints
"SIGPROF fired" to stdout via raw SYS_write.

Run with:
    cargo build -p pyroscope_cpython
    python3 kit/pyroscope_cpython/tests/test_cpython.py

Expected output (debug build):
    pyroscope_start returned: 0
    second pyroscope_start returned: 9
    Burning CPU for 3 seconds...
    SIGPROF fired
    SIGPROF fired
    ...
    done
"""
import ctypes
import os
import sys
import time


def find_library():
    """Find the built .so in target/debug or target/release."""
    base = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.abspath(os.path.join(base, "..", "..", ".."))
    for profile in ("debug", "release"):
        path = os.path.join(repo_root, "target", profile, "libpyroscope_cpython.so")
        if os.path.exists(path):
            return path
    print(
        f"ERROR: libpyroscope_cpython.so not found under {repo_root}/target/",
        file=sys.stderr,
    )
    sys.exit(1)


def burn_cpu(seconds):
    """Burn CPU time to trigger ITIMER_PROF / SIGPROF signals."""
    end = time.monotonic() + seconds
    total = 0
    while time.monotonic() < end:
        for i in range(100_000):
            total += i
    return total


def main():
    lib_path = find_library()
    print(f"Loading: {lib_path}")
    lib = ctypes.CDLL(lib_path)

    lib.pyroscope_start.restype = ctypes.c_int
    lib.pyroscope_start.argtypes = [ctypes.c_char_p, ctypes.c_char_p]

    rc = lib.pyroscope_start(b"test-app", b"http://localhost:4040")
    print(f"pyroscope_start returned: {rc}")
    if rc != 0:
        print(f"ERROR: pyroscope_start failed with code {rc}", file=sys.stderr)
        sys.exit(rc)

    # Calling again should return 9 (already running).
    rc2 = lib.pyroscope_start(b"test-app", b"http://localhost:4040")
    print(f"second pyroscope_start returned: {rc2}")
    assert rc2 == 9, f"Expected 9 (already running), got {rc2}"

    print("Burning CPU for 3 seconds...")
    burn_cpu(3)
    print("done")


if __name__ == "__main__":
    main()
