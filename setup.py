from setuptools import setup
from setuptools_rust import Binding, RustExtension

setup(
    name="sdag",
    version="0.1.0",
    rust_extensions=[
        RustExtension("sdag", binding=Binding.PyO3)
    ],
    zip_safe=False,
    python_requires=">=3.8",
)