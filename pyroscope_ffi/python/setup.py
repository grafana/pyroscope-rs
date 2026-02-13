from pathlib import Path

from setuptools import setup
from setuptools_rust import Binding, RustExtension

SCRIPT_DIR = Path(__file__).resolve().parent

setup(
    platforms="any",
    rust_extensions=[
        RustExtension(
            "pyroscope._native",
            path=str(SCRIPT_DIR / "lib" / "Cargo.toml"),
            binding=Binding.PyO3,
            debug=False,
        )
    ],
    setup_requires=["setuptools-rust>=1.8.0"],
    include_package_data=True,
    zip_safe=False,
)
