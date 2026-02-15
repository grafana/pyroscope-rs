import re
from pathlib import Path

import cffi
from cffi import recompiler as cffi_recompiler


_directive_re = re.compile(r'(?m)^\s*#.*?$')

python_dir = Path(__file__).resolve().parents[2]
src = (python_dir / 'rust/include/pyroscope_ffi.h').read_text()
dst = python_dir / 'python/pyroscope/_cffi.py'
src = _directive_re.sub('', src)

ffi = cffi.FFI()
ffi.cdef(src)
ffi.set_source('pyroscope', None)
res = cffi_recompiler.make_py_source(ffi, 'pyroscope', str(dst))
print(res)
