require 'ffi'

module Rust
  extend FFI::Library
  ffi_lib '/home/omarabid/Documents/Projects/Pyroscope/pyroscope/pyroscope_ffi/ruby/ffi_lib/target/release/libpyroscope_ffi.' + FFI::Platform::LIBSUFFIX
  attach_function :initialize_agent, [:string, :string, :int, :bool, :string], :bool
  attach_function :drop_agent, [], :bool
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

      Rust.initialize_agent(@config.application_name, @config.server_address, @config.sample_rate, @config.detect_subprocesses, tags_to_string(@config.tags))

      puts @config
    end

    def drop
      Rust.drop_agent
    end
  end
end

# convert tags object to string
def tags_to_string(tags)
  tags.map { |k, v| "#{k}=#{v}" }.join(',')
end
