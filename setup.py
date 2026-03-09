from setuptools import setup
from setuptools_rust import Binding, RustExtension

setup(
    rust_extensions=[
        RustExtension(
            "pyroscope_python_extension.pyroscope_python_extension",
            path="pyroscope_ffi/python/rust/Cargo.toml",
            binding=Binding.NoBinding,
            cargo_manifest_args=["--locked"],
        )
    ],
)
