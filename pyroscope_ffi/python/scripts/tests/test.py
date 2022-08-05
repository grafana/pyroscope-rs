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
    application_name = f'{os.getenv("PYROSCOPE_RUN_ID")}',
    server_address = "https://ingest.pyroscope.cloud",
    auth_token     = f'{os.getenv("PYROSCOPE_API_TOKEN")}',
    enable_logging =True,
    detect_subprocesses = os.getenv("PYROSCOPE_DETECT_SUBPROCESSES") == "true",
    oncpu = os.getenv("PYROSCOPE_ONCPU") == "true",
    gil_only =  os.getenv("PYROSCOPE_GIL_ONLY") == "true",
    report_pid = True,
    report_thread_id = True,
    report_thread_name = True,
    tags           = {
        "detect_subprocesses": f'{os.getenv("PYROSCOPE_DETECT_SUBPROCESSES")}',
        "oncpu": f'{os.getenv("PYROSCOPE_ONCPU")}',
        "gil_only": f'{os.getenv("PYROSCOPE_GIL_ONLY")}',
        "version": f'{os.getenv("PYTHON_VERSION")}',
        "arch": f'{os.getenv("PYROSCOPE_ARCH")}',
    }
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
