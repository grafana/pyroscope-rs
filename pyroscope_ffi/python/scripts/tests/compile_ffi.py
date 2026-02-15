import re
import cffi
from cffi import recompiler as cffi_recompiler


_directive_re = re.compile(r'(?m)^\s*#.*?$')

src = '/pyroscope_ffi/python/rust/include/pyroscope_ffi.h'
dst = '/python/python/pyroscope/_cffi.py'
src = open(src, 'r').read()
src = _directive_re.sub('', src)

ffi = cffi.FFI()
ffi.cdef(src)
ffi.set_source('pyroscope', None)
res = cffi_recompiler.make_py_source(ffi, 'pyroscope', dst)
print(res)