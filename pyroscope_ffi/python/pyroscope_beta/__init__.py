from collections import namedtuple
from cffi import FFI
ffi = FFI()

Config = namedtuple('Config', ('application_name', 'server_address', 'sample_rate', 'detect_subprocesses', 'log_level'))

R = ffi.dlopen("/home/omarabid/Documents/Projects/Pyroscope/pyroscope/pyroscope_ffi/python/ffi_lib/target/release/libpyroscope_ffi.so")

ffi.cdef("bool initialize_agent(char[], char[], int, bool, char[]);")

ffi.cdef("bool add_tag(long long, char[], char[]);")

ffi.cdef("bool remove_tag(long long, char[], char[]);")

def configure(application_name=None, server_address="http://localhost:4040", sample_rate=100, detect_subprocesses=False, log_level="info", tags=None):
    # Print all arguments
    print("Application name: {}".format(application_name))
    print("Server address: {}".format(server_address))
    print("Sample rate: {}".format(sample_rate))
    print("Detect subprocesses: {}".format(detect_subprocesses))
    print("Log level: {}".format(log_level))
    print("Tags: {}".format(tags))
    R.initialize_agent(application_name.encode("UTF-8"),
            server_address.encode("UTF-8"), sample_rate, detect_subprocesses,
            tags_to_string(tags).encode("UTF-8"))

def add_tag(thread_id, key, value):
    R.add_tag(thread_id, key.encode("UTF-8"), value.encode("UTF-8"))

def remove_tag(thread_id, key, value):
    R.remove_tag(thread_id, key.encode("UTF-8"), value.encode("UTF-8"))

# Convert a struct of tags to a string
def tags_to_string(tags):
    if tags is None:
        return ""
    return ",".join(["{}={}".format(key, value) for key, value in tags.items()])
