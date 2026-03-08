#!/usr/bin/env python3
"""
Smoke test for pyroscope_cpython cdylib.

Loads the .so via ctypes, calls pyroscope_start() with logging enabled,
then burns CPU for 20 seconds so at least one 15-second pprof flush
is triggered. Uses nested function calls to produce multi-frame stacks.

Run with:
    cargo build -p pyroscope_cpython
    python3.14 kit/pyroscope_cpython/tests/test_cpython.py

Expected: pyroscope_start returns 0, flush log line appears at ~15s,
profile is sent to Pyroscope (if running on localhost:4040).
"""
import asyncio
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


def inner_work():
    """Inner function to produce deeper stacks."""
    total = 0
    for i in range(100_000):
        total += i
    return total


def outer_work():
    """Outer function that calls inner_work."""
    return inner_work()


def burn_cpu(seconds):
    """Burn CPU time to trigger ITIMER_PROF / SIGPROF signals."""
    end = time.monotonic() + seconds
    total = 0
    while time.monotonic() < end:
        total += outer_work()
    return total


# Function with Unicode characters in name: U+00E9 (é), U+00E0 (à), U+00FC (ü).
# These are Latin-1 range (>0x7F), so CPython stores the name as a non-ASCII
# compact string (kind=1, ascii=0). This exercises the non-ASCII reading path.
def burn_cpu_éàü(seconds):
    end = time.monotonic() + seconds
    total = 0
    while time.monotonic() < end:
        for i in range(100_000):
            total += i
    return total


# Function with CJK characters: U+4E16 (世), U+754C (界).
# These require UCS2 storage in CPython (kind=2).
def burn_cpu_世界(seconds):
    end = time.monotonic() + seconds
    total = 0
    while time.monotonic() < end:
        for i in range(100_000):
            total += i
    return total


# ── Async workload ───────────────────────────────────────────────────────────
# These exercise asyncio profiling. Running coroutines appear in SIGPROF
# samples (their frames are in the thread's frame chain). Suspended coroutines
# (e.g. awaiting asyncio.sleep) are NOT captured by SIGPROF — they require
# walking the cr_await chain from the asyncio task list.


async def async_cpu_work():
    """Burns CPU inside a coroutine — will appear in SIGPROF samples."""
    total = 0
    for i in range(500_000):
        total += i
    return total


async def async_inner():
    """Inner async function to create deeper async call stacks."""
    return await async_cpu_work()


async def async_outer():
    """Outer async function: async_outer > async_inner > async_cpu_work."""
    return await async_inner()


async def async_io_work():
    """Sleeps briefly — this is a suspended task, NOT visible in SIGPROF."""
    await asyncio.sleep(0.01)


async def async_mixed_workload():
    """Run a mix of CPU-bound and IO-bound async tasks concurrently."""
    tasks = []
    for _ in range(5):
        tasks.append(asyncio.create_task(async_outer()))
        tasks.append(asyncio.create_task(async_io_work()))
    await asyncio.gather(*tasks)


async def async_main():
    """Main async entry point — runs for ~5 seconds."""
    end = time.monotonic() + 5
    while time.monotonic() < end:
        await async_mixed_workload()


def main():
    lib_path = find_library()
    print(f"Loading: {lib_path}")
    lib = ctypes.CDLL(lib_path)

    lib.pyroscope_start.restype = ctypes.c_int
    lib.pyroscope_start.argtypes = [
        ctypes.c_char_p,  # app_name
        ctypes.c_char_p,  # server_url
        ctypes.c_int,     # num_shards (0 = default)
        ctypes.c_int,     # log_enabled
    ]

    # num_shards=0 (use default 16), log_enabled=1
    rc = lib.pyroscope_start(b"test-app", b"http://localhost:4040", 0, 1)
    print(f"pyroscope_start returned: {rc}")
    if rc != 0:
        print(f"ERROR: pyroscope_start failed with code {rc}", file=sys.stderr)
        sys.exit(rc)

    # Calling again should return 9 (already running).
    rc2 = lib.pyroscope_start(b"test-app", b"http://localhost:4040", 0, 0)
    print(f"second pyroscope_start returned: {rc2}")
    assert rc2 == 9, f"Expected 9 (already running), got {rc2}"

    print("Burning CPU for 20 seconds (flush expected at ~15s)...")
    burn_cpu(20)
    print("done with ASCII burn")

    # Burn CPU with unicode-named functions to test non-ASCII string reading.
    # The profiler should resolve these names in the debug output.
    print("Burning CPU with Latin-1 function name (burn_cpu_éàü) for 3 seconds...")
    burn_cpu_éàü(3)
    print("done with Latin-1 burn")

    print("Burning CPU with CJK function name (burn_cpu_世界) for 3 seconds...")
    burn_cpu_世界(3)
    print("done with CJK burn")

    # Async workload: running coroutines show up in SIGPROF, suspended ones don't.
    print("Running asyncio workload for 5 seconds...")
    asyncio.run(async_main())
    print("done with asyncio workload")


if __name__ == "__main__":
    main()
