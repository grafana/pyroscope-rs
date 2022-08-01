# coding: utf-8
# frozen_string_literal: true

require 'ffi'

module Pyroscope
  module Rust
    extend FFI::Library
    ffi_lib File.expand_path(File.dirname(__FILE__)) + "/rbspy/rbspy.#{RbConfig::CONFIG["DLEXT"]}"
    attach_function :initialize_logging, [:int], :bool
    attach_function :initialize_agent, [:string, :string, :string, :int, :bool, :bool, :bool, :bool, :string], :bool
    attach_function :add_thread_tag, [:uint64, :string, :string], :bool
    attach_function :remove_thread_tag, [:uint64, :string, :string], :bool
    attach_function :add_global_tag, [:string, :string], :bool
    attach_function :remove_global_tag, [:string, :string], :bool
    attach_function :drop_agent, [], :bool
  end

  module Utils
    extend FFI::Library
    ffi_lib File.expand_path(File.dirname(__FILE__)) + "/thread_id/thread_id.#{RbConfig::CONFIG["DLEXT"]}"
    attach_function :thread_id, [], :uint64
  end

  Config = Struct.new(:application_name, :app_name, :server_address, :auth_token, :log_level, :sample_rate, :detect_subprocesses, :oncpu, :report_pid, :report_thread_id, :tags) do
    def initialize(*)
      self.application_name = ''
      self.server_address = 'http://localhost:4040'
      self.auth_token = ''
      self.sample_rate = 100
      self.detect_subprocesses = false
      self.oncpu = true
      self.report_pid = false
      self.report_thread_id = false
      self.log_level = 'error'
      self.tags = {}
      super
    end
  end

  class << self
    def configure
      @config = Config.new

      # Pass config to the block
      yield @config

      # Determine Logging level (kinda like an enum).
      case @config.log_level
      when 'trace'
        @log_level = 10
      when 'debug'
        @log_level = 20
      when 'info'
        @log_level = 30
      when 'warn'
        @log_level = 40
      when 'error'
        @log_level = 50
      else
        @log_level = 50
      end

      # Initialize Logging
      Rust.initialize_logging(@log_level)
      

      # initialize Pyroscope Agent
      Rust.initialize_agent(
        @config.app_name || @config.application_name || "",
        @config.server_address || "",
        @config.auth_token || "",
        @config.sample_rate || 100,
        @config.detect_subprocesses || false,
        @config.oncpu || false,
        @config.report_pid || false,
        @config.report_thread_id || false,
        tags_to_string(@config.tags || {})
      )
    end

    def tag_wrapper(tags)
      tid = thread_id
      _add_tags(tid, tags)
      begin
        yield
      ensure
        _remove_tags(tid, tags)
      end
    end

    def tag(tags)
      warn("deprecated. Use `Pyroscope.tag_wrapper` instead.")
    end

    def remove_tags(*tags)
      warn("deprecated. Use `Pyroscope.tag_wrapper` instead.")
    end

    # convert tags object to string
    def tags_to_string(tags)
      tags.map { |k, v| "#{k}=#{v}" }.join(',')
    end

    # get thread id
    def thread_id
      return Utils.thread_id
    end

    # add tags
    def _add_tags(thread_id, tags)
      tags.each do |tag_name, tag_value|
        Rust.add_thread_tag(thread_id, tag_name.to_s, tag_value.to_s)
      end
    end

    # remove tags
    def _remove_tags(thread_id, tags)
      tags.each do |tag_name, tag_value|
        Rust.remove_thread_tag(thread_id, tag_name.to_s, tag_value.to_s)
      end
    end

    def shutdown
      Rust.drop_agent
    end
  end
end
