import os
import sys
from setuptools import find_packages, setup
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
os.chdir(SCRIPT_DIR)
LIB_DIR = str(SCRIPT_DIR / "lib")

def build_native(spec):
    # Step 1: build the rust library
    build = spec.add_external_build(
        cmd=['cargo', 'build', '--release'],
        path=LIB_DIR
    )

    def find_dylib():
        cargo_target = os.environ.get('CARGO_BUILD_TARGET')
        if cargo_target:
            in_path = 'target/%s/release' % (cargo_target)
        else:
            in_path = 'target/release'
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
    setup_requires=["milksnakex>=0.1.6"],
)
