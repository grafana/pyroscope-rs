import os
import sys
from setuptools import find_packages, setup
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
os.chdir(SCRIPT_DIR)
LIB_DIR = str(SCRIPT_DIR / "lib")

def build_native(spec):
    cargo_profile_debug = os.environ.get('CARGO_PROFILE') == 'debug'
    cargo_profile_path = 'debug' if cargo_profile_debug else 'release'
    cargo_profile_arg = [] if cargo_profile_debug else ['--release']

    # Step 1: build the rust library
    build = spec.add_external_build(
        cmd=['cargo', 'build', '-p' 'pyroscope_ffi'] + cargo_profile_arg,
        path=LIB_DIR
    )

    def find_dylib():
        cargo_target = os.environ.get('CARGO_BUILD_TARGET')
        if cargo_target:
            in_path = '../../../target/%s/%s' % (cargo_target, cargo_profile_path)
        else:
            in_path = '../../../target/%s' % (cargo_profile_path)
        print("find_dylib  in_path: %s" % (in_path))
        return build.find_dylib('pyroscope_ffi', in_path=in_path)

    # Step 2: package the compiled library
    rtld_flags = ["NOW"]
    if sys.platform == "darwin":
        rtld_flags.append("NODELETE")

    spec.add_cffi_module(module_path='pyroscope._native',
            dylib=find_dylib,
            header_filename=lambda:
            build.find_header('pyroscope_ffi.h',in_path='include'),
            rtld_flags=rtld_flags,
    )

setup(
    platforms="any",
    milksnake_tasks=[build_native],
    setup_requires=["pyromilksnakex==0.1.8"],
)
