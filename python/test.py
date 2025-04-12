from ctypes import *
libpath = "/home/korniltsev/p/pyroscope-rs/target/debug/libpython.so"

cdll.LoadLibrary(libpath)