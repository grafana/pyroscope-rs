from pathlib import Path

from setuptools import setup
from setuptools_rust import Binding, RustExtension

SCRIPT_DIR = Path(__file__).resolve().parent

setup(
    platforms="any",
    rust_extensions=[
        RustExtension(
            "pyroscope._native_lib",
            path=str(SCRIPT_DIR / "lib" / "Cargo.toml"),
            binding=Binding.NoBinding,
            debug=False,
        )
    ],
    setup_requires=["setuptools-rust>=1.8.0", "cffi>=1.6.0", "pycparser"],
    include_package_data=True,
    zip_safe=False,
)
