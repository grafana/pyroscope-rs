import re
import cffi
from cffi import recompiler as cffi_recompiler


_directive_re = re.compile(r'(?m)^\s*#.*?$')

src = '/home/korniltsev/pyroscope-rs/pyroscope_ffi/python/lib/include/pyroscope_ffi.h'
dst = '/home/korniltsev/pyroscope-rs/pyroscope_ffi/python/python/pyroscope/_cffi.py'
src = open(src, 'r').read()
src = _directive_re.sub('', src)

ffi = cffi.FFI()
ffi.cdef(src)
ffi.set_source('pyroscope', None)
res = cffi_recompiler.make_py_source(ffi, 'pyroscope', dst)
print(res)