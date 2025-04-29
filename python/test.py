from ctypes import *

# todo use pyo3
libpath = "/home/korniltsev/p/pyroscope-rs/target/debug/libpython.so"

cdll.LoadLibrary(libpath)


def fib(n):
    if n <= 1:
        return 1
    return fib(n-1) + fib(n-2)

while True:
    fib(42)