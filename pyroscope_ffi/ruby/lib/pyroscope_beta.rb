require 'ffi'

module Rust
  extend FFI::Library
  ffi_lib File.expand_path(File.dirname(__FILE__)) + "/rbspy/rbspy.#{RbConfig::CONFIG["DLEXT"]}"
  attach_function :initialize_agent, [:string, :string, :int, :bool, :string], :bool
  attach_function :add_tag, [:uint64, :string, :string], :bool
  attach_function :remove_tag, [:uint64, :string, :string], :bool
  attach_function :drop_agent, [], :bool
end

module Utils
     extend FFI::Library
     ffi_lib File.expand_path(File.dirname(__FILE__)) + "/thread_id/thread_id.#{RbConfig::CONFIG["DLEXT"]}"
     attach_function :thread_id, [], :uint64
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

      Rust.initialize_agent(
        @config.application_name,
        @config.server_address,
        @config.sample_rate,
        @config.detect_subprocesses,
        tags_to_string(@config.tags)
      )

      puts @config
    end

    def tag_wrapper(tags)
      add_tags(tags)
      begin
        yield
      ensure
        remove_tags(tags)
      end
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

# get thread id
def thread_id
  return Utils.thread_id()
end

# add tags
def add_tags(tags)
  tags.each do |tag_name, tag_value|
    thread_id = thread_id()
    Rust.add_tag(thread_id, tag_name.to_s, tag_value.to_s)
  end
end

# remove tags
def remove_tags(tags)
  tags.each do |tag_name, tag_value|
    thread_id = thread_id()
    Rust.remove_tag(thread_id, tag_name.to_s, tag_value.to_s)
  end
end
