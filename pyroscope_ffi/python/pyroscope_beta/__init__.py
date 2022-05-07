import threading
from collections import namedtuple
from pyroscope_beta._native import ffi, lib
from contextlib import contextmanager

Config = namedtuple('Config', ('application_name', 'server_address', 'sample_rate', 'detect_subprocesses', 'log_level'))

def configure(application_name=None, server_address="http://localhost:4040", sample_rate=100, detect_subprocesses=False, log_level="info", tags=None): 
    lib.initialize_agent(application_name.encode("UTF-8"),
            server_address.encode("UTF-8"), sample_rate, detect_subprocesses,
            tags_to_string(tags).encode("UTF-8"))

def add_thread_tag(thread_id, key, value):
    lib.add_thread_tag(thread_id, key.encode("UTF-8"), value.encode("UTF-8"))

def remove_thread_tag(thread_id, key, value):
    lib.remove_thread_tag(thread_id, key.encode("UTF-8"), value.encode("UTF-8"))

def add_global_tag(thread_id, key, value):
    lib.add_global_tag(key.encode("UTF-8"), value.encode("UTF-8"))

def remove_global_tag(key, value):
    lib.remove_global_tag(key.encode("UTF-8"), value.encode("UTF-8"))

# Convert a struct of tags to a string
def tags_to_string(tags):
    if tags is None:
        return ""
    return ",".join(["{}={}".format(key, value) for key, value in tags.items()])

@contextmanager
def tag_wrapper(tags):
    for key, value in tags.items():
        lib.add_thread_tag(threading.get_ident(), key.encode("UTF-8"), value.encode("UTF-8"))
    try:
        yield
    finally:
        for key, value in tags.items():
            lib.remove_thread_tag(threading.get_ident(), key.encode("UTF-8"), value.encode("UTF-8"))
