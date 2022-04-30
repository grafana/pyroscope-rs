Gem::Specification.new do |s|
  s.name = 'pyroscope_beta'
  s.version = Pyroscope::VERSION
  s.summary = 'Pyroscope Beta'
  s.description = 'Pyroscope FFI Integration for Ruby'
  s.authors = ['Pyroscope Team']
  s.email = ['contact@pyroscope.io']
  s.files = `git ls-files`.split("\n")
  s.homepage = 'https://pyroscope.io'
  s.license = 'Apache-2.0'

  s.platform = Gem::Platform::RUBY

  s.add_development_dependency 'rake', '~> 10.0'

  s.add_dependency 'ffi', '~> 1.9'
end
