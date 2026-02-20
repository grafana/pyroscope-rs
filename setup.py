import os

from setuptools import setup
from setuptools_rust import Binding, RustExtension

cargo_args = ["--locked"]
features = []

if os.getenv("CARGO_NO_DEFAULT_FEATURES"):
    cargo_args.append("--no-default-features")

extra_features = os.getenv("CARGO_FEATURES")
if extra_features:
    features = extra_features.split(",")

setup(
    rust_extensions=[
        RustExtension(
            "pyroscope_python_extension.pyroscope_python_extension",
            path="pyroscope_ffi/python/rust/Cargo.toml",
            binding=Binding.NoBinding,
            cargo_manifest_args=cargo_args,
            features=features,
        )
    ],
)
