# coding: utf-8
# frozen_string_literal: true

begin
  require File.expand_path(File.join(File.dirname(__FILE__), "lib/pyroscope/version"))
rescue LoadError
  puts "WARNING: Could not load Pyroscope::VERSION"
end

Gem::Specification.new do |s|
  s.name = 'pyroscope'
  s.version = Pyroscope::VERSION
  s.summary = 'Pyroscope'
  s.description = 'Pyroscope FFI Integration for Ruby'
  s.authors = ['Pyroscope Team']
  s.email = ['contact@pyroscope.io']
  s.homepage = 'https://pyroscope.io'
  s.license = 'Apache-2.0'
  s.metadata = {
    "homepage_uri" => "https://pyroscope.io",
    "bug_tracker_uri" => "https://github.com/pyroscope-io/pyroscope-rs/issues",
    "documentation_uri" => "https://pyroscope.io/docs/ruby/",
    "changelog_uri" => "https://github.com/pyroscope-io/pyroscope-rs/tree/main/pyroscope_ffi/ruby/CHANGELOG.md",
    "source_code_uri" => "https://github.com/pyroscope-io/pyroscope-rs/tree/main/pyroscope_ffi/ruby",
  }

  # Specify which files should be added to the gem when it is released.
  # The `git ls-files -z` loads the files in the RubyGem that have been added into git.
  #s.files = Dir.chdir(__dir__) do
    #`git ls-files -z`.split("\x0").reject do |f|
      #(f == __FILE__) || f.match(%r{\A(?:(?:bin|test|spec|features)/|\.(?:git|travis|circleci)|appveyor)})
    #end
  #end
#   s.files = `git ls-files -z`.split("\0").reject { |f| f =~ /^(\.|G|spec|Rakefile)/ }
  s.files = [
    "Gemfile",
    "Gemfile.lock",
    "LICENSE",
#     "Makefile",
    "README.md",
#     "Rakefile",
    "ext/rbspy/Cargo.toml",
    "ext/rbspy/Rakefile",
    "ext/rbspy/build.rs",
    "ext/rbspy/cbindgen.toml",
    "ext/rbspy/extconf.rb",
    "ext/rbspy/include/rbspy.h",
    "ext/rbspy/src/lib.rs",
    "ext/thread_id/Cargo.toml",
    "ext/thread_id/Rakefile",
    "ext/thread_id/build.rs",
    "ext/thread_id/cbindgen.toml",
    "ext/thread_id/extconf.rb",
    "ext/thread_id/include/thread_id.h",
    "ext/thread_id/src/lib.rs",
    "lib/pyroscope.rb",
    "lib/pyroscope/version.rb",
    "pyroscope.gemspec",
#     "scripts/tests/test.rb",
  ]
  s.platform = Gem::Platform::RUBY

  s.required_ruby_version = ">= 1.9.3"

  s.extensions = ['ext/rbspy/extconf.rb', 'ext/thread_id/extconf.rb']

  s.add_dependency 'ffi'

  s.add_development_dependency 'bundler'
  s.add_development_dependency 'rake', '~> 13.0'
end
