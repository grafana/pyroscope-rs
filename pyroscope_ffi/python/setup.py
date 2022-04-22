from setuptools import setup, find_packages

def build_native(spec):
    # Step 1: build the rust library
    build = spec.add_external_build(
        cmd=['cargo', 'build', '--release'],
        path='./lib'
    )

    # Step 2: package the compiled library
    spec.add_cffi_module(module_path='lib._lowlevel',
            dylib=lambda: build.find_dylib('pyroscope_ffi', in_path='target/release'),
            header_filename=lambda: build.find_header('libpyroscope_ffi.h', in_path='include'),
            rtld_flags=['NOW', 'NODELETE']
    )

# Long description
with open('README.md', 'r', encoding='utf-8') as f:
    long_description = f.read()



setup(
    name='pyroscope_beta',
    version='0.1.0',
    author='Abid Omar',
    description='Pyroscope Python integration',
    long_description=long_description,
    long_description_content_type='text/markdown',
    packages=find_packages(),
    include_package_data=True,
    zip_safe=False,
    platforms='any',
    setup_requires=['milksnakex'],
    install_requires=['milksnakex'],
    milksnake_tasks=[
        build_native,
    ]
)
