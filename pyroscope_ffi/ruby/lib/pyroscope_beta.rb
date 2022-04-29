require 'ffi'
require 'fiddle'

$libm = Fiddle.dlopen('/home/omarabid/Documents/Projects/Pyroscope/pyroscope/pyroscope_ffi/ruby/ext/thread_id/target/release/libthread_id.so')


module Rust
  extend FFI::Library
  ffi_lib '/home/omarabid/Documents/Projects/Pyroscope/pyroscope/pyroscope_ffi/ruby/ffi_lib/target/release/libpyroscope_ffi.' + FFI::Platform::LIBSUFFIX
  attach_function :initialize_agent, [:string, :string, :int, :bool, :string], :bool
  attach_function :add_tag, [:uint64, :string, :string], :bool
  attach_function :remove_tag, [:uint64, :string, :string], :bool
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
    def add_tag(thread_id, tag_name, tag_value)
      Rust.add_tag(thread_id, tag_name, tag_value)
    end
    def remove_tag(thread_id, tag_name, tag_value)
      Rust.remove_tag(thread_id, tag_name, tag_value)
    end

    def drop
      Rust.drop_agent
    end
    def thread_id
      thread_id = Fiddle::Function.new($libm['thread_id'], [], Fiddle::TYPE_INT64_T)
      thread_id.call.to_s
    end
  end
end

# convert tags object to string
def tags_to_string(tags)
  tags.map { |k, v| "#{k}=#{v}" }.join(',')
end
