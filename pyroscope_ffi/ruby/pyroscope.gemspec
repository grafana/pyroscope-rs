require './lib/pyroscope/version'

Gem::Specification.new do |s|
  s.name = 'pyroscope_beta'
  s.version = Pyroscope::VERSION
  s.summary = 'Pyroscope Beta'
  s.description = 'Pyroscope FFI Integration for Ruby'
  s.authors = ['Pyroscope Team']
  s.email = ['contact@pyroscope.io']
  s.files = ["lib/pyroscope_beta.rb", "ffi_lib/target/release/libpyroscope_ffi.so"]
  s.homepage = 'https://pyroscope.io'
  s.license = 'Apache-2.0'
end
