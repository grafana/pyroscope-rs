require_relative "lib/pyroscope/version"

Gem::Specification.new do |s|
  s.name = 'pyroscope_beta'
  s.version = Pyroscope::VERSION
  s.summary = 'Pyroscope Beta'
  s.description = 'Pyroscope FFI Integration for Ruby'
  s.authors = ['Pyroscope Team']
  s.email = ['contact@pyroscope.io']
  s.homepage = 'https://pyroscope.io'
  s.license = 'Apache-2.0'

  # Specify which files should be added to the gem when it is released.
  # The `git ls-files -z` loads the files in the RubyGem that have been added into git.
  s.files = Dir.chdir(__dir__) do
    `git ls-files -z`.split("\x0").reject do |f|
      (f == __FILE__) || f.match(%r{\A(?:(?:bin|test|spec|features)/|\.(?:git|travis|circleci)|appveyor)})
    end
  end

  s.platform = Gem::Platform::RUBY
  s.require_paths = ['lib']

  s.extensions = ['ext/rbspy/extconf.rb', 'ext/thread_id/extconf.rb']

  s.add_dependency 'ffi', '~> 1.9'
  s.add_dependency 'fiddle', '~> 1.1'
  s.add_dependency 'rake', '>= 10.0'
end
