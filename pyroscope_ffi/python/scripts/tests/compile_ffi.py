import os
import re
import cffi
from cffi import recompiler as cffi_recompiler


_directive_re = re.compile(r'(?m)^\s*#.*?$')

_script_dir = os.path.dirname(os.path.abspath(__file__))
_python_ffi_dir = os.path.normpath(os.path.join(_script_dir, '..', '..'))

src = os.path.join(_python_ffi_dir, 'rust', 'include', 'pyroscope_ffi.h')
dst = os.path.join(_python_ffi_dir, 'python', 'pyroscope', '_cffi.py')
src = open(src, 'r').read()
src = _directive_re.sub('', src)

ffi = cffi.FFI()
ffi.cdef(src)
ffi.set_source('pyroscope', None)
res = cffi_recompiler.make_py_source(ffi, 'pyroscope', dst)
print(res)