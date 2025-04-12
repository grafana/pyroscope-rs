from ctypes import *
libpath = "/home/korniltsev/p/pyroscope-rs/target/debug/libkit.so"

cdll.LoadLibrary(libpath)