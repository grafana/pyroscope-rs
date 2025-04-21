from ctypes import *

# todo use pyo3
libpath = "/home/korniltsev/p/pyroscope-rs/target/debug/libpython.so"

cdll.LoadLibrary(libpath)