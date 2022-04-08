from collections import namedtuple
from cffi import FFI
ffi = FFI()

Config = namedtuple('Config', ('application_name', 'server_address', 'sample_rate', 'detect_subprocesses', 'log_level'))

R = ffi.dlopen("/home/omarabid/Documents/Projects/Pyroscope/pyroscope/pyroscope_ffi/python/ffi_lib/target/release/libpyroscope_ffi.so")

ffi.cdef("bool initialize_agent();")

def configure(application_name=None, server_address="http://localhost:4040", sample_rate=100, detect_subprocesses=False, log_level="info", tags=None):
    # Print all arguments
    print("Application name: {}".format(application_name))
    print("Server address: {}".format(server_address))
    print("Sample rate: {}".format(sample_rate))
    print("Detect subprocesses: {}".format(detect_subprocesses))
    print("Log level: {}".format(log_level))
    print("Tags: {}".format(tags))
    R.initialize_agent()
