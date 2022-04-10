require 'ffi'

module Rust
  extend FFI::Library
  ffi_lib '/home/omarabid/Documents/Projects/Pyroscope/pyroscope/pyroscope_ffi/ruby/ffi_lib/target/release/libpyroscope_ffi.' + FFI::Platform::LIBSUFFIX
  attach_function :initialize_agent, [:string, :string, :int, :bool], :bool
end

module Pyroscope
  Config = Struct.new(:application_name, :server_address, :sample_rate, :detect_subprocesses, :log_level, :tags) do
    def initialize(*)
      super
      self.application_name ||= '' 
      self.server_address ||= 'http://localhost:4040'
      self.sample_rate ||= 100
      self.detect_subprocesses ||= true
      self.log_level ||= 'info'
      self.tags ||= []
    end
  end

  class << self
    def configure
      @config = Config.new

      # Pass config to the block
      yield @config

      Rust.initialize_agent(@config.application_name, @config.server_address, @config.sample_rate, @config.detect_subprocesses)

      puts @config
    end
  end
end
