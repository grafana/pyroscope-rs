#!/usr/bin/env python3
import hashlib
import os
import threading
import logging

import pyroscope


# Set python logging level to DEBUG
logger = logging.getLogger()
logger.setLevel(logging.DEBUG)

# Configure Pyroscope
pyroscope.configure(
    application_name = f'{os.getenv("PYROSCOPE_RUN_ID")}-x86-64-linux-onpcu',
    server_address = "https://ingest.pyroscope.cloud",
    auth_token     = os.getenv("PYROSCOPE_API_TOKEN"),
    enable_logging=True,
    detect_subprocesses=False,
    oncpu=True,
    gil_only=False,
    report_pid=True,
    report_thread_id=True,
    report_thread_name=True,
)


def hash(string):
    string = string.encode()
    string = hashlib.sha256(string).hexdigest()

    return string

def multihash(string):
    for i in range(0, 25510055):
        string = hash(string)
    return string

def multihash2(string):
    for i in range(0, 25510055):
        string = hash(string)
    return string

thread_1 = threading.Thread(target=multihash, args=('abc',))
thread_2 = threading.Thread(target=multihash2, args=('abc',))

thread_1.start()
thread_2.start()

thread_1.join()
thread_2.join()

pyroscope.shutdown()
