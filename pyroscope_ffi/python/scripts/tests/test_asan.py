"""
Simple script that configures pyroscope and burns CPU.
The ASAN OOB trigger in the profiler backend will fire after 50 samples.

Usage:
    ASAN_OPTIONS=detect_leaks=0 python3.11 test_asan.py
"""

import hashlib
import logging
import time

import pyroscope

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

pyroscope.configure(
    application_name="pyroscopers.python.asan.test",
    server_address="http://localhost:4040",
    enable_logging=True,
    oncpu=True,
    gil_only=True,
    report_pid=True,
    report_thread_id=True,
    report_thread_name=True,
    tags={
        "test": "asan",
    },
)

logger.info("pyroscope configured, burning CPU...")

data = "burn-cpu"
while True:
    data = hashlib.sha256(data.encode()).hexdigest()
