from cffi import FFI
from pathlib import Path
import sys

ffi = FFI()
ffi.cdef(
    """
    typedef enum {
        LastInstruction = 0,
        First = 1,
        NoLine = 2,
    } LineNo;

    _Bool initialize_logging(unsigned int logging_level);
    _Bool initialize_agent(
        const char* application_name,
        const char* server_address,
        const char* basic_auth_username,
        const char* basic_auth_password,
        unsigned int sample_rate,
        _Bool oncpu,
        _Bool gil_only,
        _Bool report_pid,
        _Bool report_thread_id,
        _Bool report_thread_name,
        const char* tags,
        const char* tenant_id,
        const char* http_headers_json,
        LineNo line_no
    );
    _Bool drop_agent(void);
    _Bool add_thread_tag(const char* key, const char* value);
    _Bool remove_thread_tag(const char* key, const char* value);
    """
)


def _find_native_library() -> Path:
    package_dir = Path(__file__).resolve().parent
    candidates = []
    for pattern in ("_native_lib*.so", "_native_lib*.dylib", "_native_lib*.dll", "_native_lib*.pyd"):
        candidates.extend(package_dir.glob(pattern))

    if not candidates and sys.platform.startswith("linux"):
        candidates.extend(package_dir.glob("libpyroscope_ffi*.so"))

    if not candidates:
        raise ImportError("Unable to locate pyroscope native library built by setuptools-rust")

    return candidates[0]


lib = ffi.dlopen(str(_find_native_library()))
