
__all__ = ['lib', 'ffi']

import os
from ._cffi import ffi

lib = ffi.dlopen(os.path.join(os.path.dirname(__file__), '../pyroscope_python_extension', 'pyroscope_python_extension.abi3.so'))
del os
