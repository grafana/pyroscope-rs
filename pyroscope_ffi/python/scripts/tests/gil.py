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
    app_name       = "run.python.app",
    server_address = "http://localhost:4040",
    enable_logging=True,
    detect_subprocesses=False,
    oncpu=False,
    gil_only=True,
    report_pid=True,
    report_thread_id=True,
    report_thread_name=True,
)


def hash(string):
    string = string.encode()
    string = hashlib.sha256(string).hexdigest()

    return string

def multihash(string):
    for i in range(0, 55510055):
        string = hash(string)
    return string

def multihash2(string):
    for i in range(0, 55510055):
        string = hash(string)
    return string

thread_1 = threading.Thread(target=multihash, args=('abc',))
thread_2 = threading.Thread(target=multihash2, args=('abc',))

thread_1.start()
thread_2.start()

thread_1.join()
thread_2.join()

pyroscope.shutdown()
