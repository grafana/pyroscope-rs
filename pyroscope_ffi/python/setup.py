from pathlib import Path
import sys

from setuptools import setup

SCRIPT_DIR = Path(__file__).resolve().parent


def is_sdist_build() -> bool:
    return "sdist" in sys.argv


def build_rust_extensions():
    try:
        from setuptools_rust import Binding, RustExtension
    except ModuleNotFoundError:
        if is_sdist_build():
            # Allow source distribution creation without setuptools-rust installed.
            return []
        raise

    return [
        RustExtension(
            "pyroscope._native_lib",
            path=str(SCRIPT_DIR / "lib" / "Cargo.toml"),
            binding=Binding.NoBinding,
            debug=False,
        )
    ]


setup(
    platforms="any",
    rust_extensions=build_rust_extensions(),
    include_package_data=True,
    zip_safe=False,
)
