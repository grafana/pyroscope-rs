from setuptools import setup, find_packages

def build_native(spec):
    # Step 1: build the rust library
    build = spec.add_external_build(
        cmd=['cargo', 'build', '--release'],
        path='./ffi_lib'
    )

    # Step 2: package the compiled library



setup(
    name='pyroscope_beta',
    version='0.1.0',
    packages=find_packages(),
    include_package_data=True,
    zip_safe=False,
    platforms='any',
    install_requires=[
    ],
    milksnake_tasks=[
        build_native,
    ]
)
