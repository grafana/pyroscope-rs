import cffi


ffi = cffi.FFI()
ffi.cdef(
    """
int cffi_func(void);
"""
)
ffi.set_source(
    "pyroscope._cffi",
    None,
)
