require_relative "lib/pyroscope/version"

Gem::Specification.new do |s|
  s.name = 'pyroscope'
  s.version = Pyroscope::VERSION
  s.summary = 'Pyroscope'
  s.description = 'Pyroscope FFI Integration for Ruby'
  s.authors = ['Pyroscope Team']
  s.email = ['contact@pyroscope.io']
  s.homepage = 'https://pyroscope.io'
  s.license = 'Apache-2.0'

  # Specify which files should be added to the gem when it is released.
  # The `git ls-files -z` loads the files in the RubyGem that have been added into git.
  #s.files = Dir.chdir(__dir__) do
    #`git ls-files -z`.split("\x0").reject do |f|
      #(f == __FILE__) || f.match(%r{\A(?:(?:bin|test|spec|features)/|\.(?:git|travis|circleci)|appveyor)})
    #end
  #end
  s.files = `git ls-files -z`.split("\0").reject { |f| f =~ /^(\.|G|spec|Rakefile)/ }

  s.platform = Gem::Platform::RUBY

  s.required_ruby_version = ">= 2.5.9"

  s.extensions = ['ext/rbspy/extconf.rb', 'ext/thread_id/extconf.rb']

  s.add_dependency 'ffi'

  s.add_development_dependency 'bundler'
  s.add_development_dependency 'rake', '~> 13.0'
end
