# coding: utf-8
# frozen_string_literal: true

require 'ffi'
require 'json'

module Pyroscope
  module Rust
    extend FFI::Library
    ffi_lib File.expand_path(File.dirname(__FILE__)) + "/rbspy/rbspy.#{RbConfig::CONFIG["DLEXT"]}"
    attach_function :initialize_logging, [:int], :bool
    attach_function :initialize_agent, [:string, :string, :string, :string, :string, :int, :bool, :bool, :bool, :string, :string, :string, :string, :string], :bool
    attach_function :add_thread_tag, [:string, :string], :bool
    attach_function :remove_thread_tag, [:string, :string], :bool
    attach_function :add_global_tag, [:string, :string], :bool
    attach_function :remove_global_tag, [:string, :string], :bool
    attach_function :drop_agent, [], :bool
  end

  if defined?(::Rails::Engine)
    class Engine < ::Rails::Engine
      config.after_initialize do
        next unless ::Pyroscope.current_config && ::Pyroscope.current_config.autoinstrument_rails

        ::Pyroscope.initialize_rails_hooks
      end
    end
  end

  Config = Struct.new(
    :application_name,
    :app_name,
    :server_address,
    :auth_token,
    :basic_auth_username,
    :basic_auth_password,
    :log_level,
    :sample_rate,
    :detect_subprocesses,
    :oncpu,
    :report_pid,
    :report_thread_id,
    :tags,
    :compression,
    :report_encoding,
    :autoinstrument_rails,
    :tenant_id,
    :http_headers,
  ) do
    def initialize(*)
      super
      # defaults:
      self.application_name = ''
      self.server_address = 'http://localhost:4040'
      self.auth_token = ''
      self.basic_auth_username = ''
      self.basic_auth_password = ''
      self.sample_rate = 100
      self.detect_subprocesses = false
      self.oncpu = true
      self.report_pid = false
      self.report_thread_id = false
      self.log_level = 'error'
      self.tags = {}
      self.compression = 'gzip'
      self.report_encoding = 'pprof'
      self.autoinstrument_rails = true
      self.tenant_id = ''
      self.http_headers = {}
    end
  end

  class << self
    def current_config
      @config
    end

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

      Rust.initialize_logging(@log_level)

      Rust.initialize_agent(
        # these are defaults in case user-provided values are nil:
        @config.app_name || @config.application_name || "",
        @config.server_address || "",
        @config.auth_token || "",
        @config.basic_auth_username || "",
        @config.basic_auth_password || "",
        @config.sample_rate || 100,
        @config.oncpu || false,
        @config.report_pid || false,
        @config.report_thread_id || false,
        tags_to_string(@config.tags || {}),
        @config.compression || "",
        @config.report_encoding || "pprof",
        @config.tenant_id || "",
        http_headers_to_json(@config.http_headers || {})
      )
    end

    def initialize_rails_hooks
      block = lambda do |ctrl, action|
        Pyroscope.tag_wrapper({
          "action" => "#{ctrl.controller_name}/#{ctrl.action_name}"
        }, &action)
      end

      ActionController::API.__send__(:around_action, block) if defined? ActionController::API
      ActionController::Base.__send__(:around_action, block) if defined? ActionController::Base
    end

    def tag_wrapper(tags)
      _add_tags(tags)
      begin
        yield
      ensure
        _remove_tags(tags)
      end
    end

    def tag(tags)
      warn("deprecated. Use `Pyroscope.tag_wrapper` instead.")
    end

    def remove_tags(*tags)
      warn("deprecated. Use `Pyroscope.tag_wrapper` instead.")
    end

    def _add_tags(tags)
      tags.each do |tag_name, tag_value|
        Rust.add_thread_tag(tag_name.to_s, tag_value.to_s)
      end
    end

    def _remove_tags(tags)
      tags.each do |tag_name, tag_value|
        Rust.remove_thread_tag(tag_name.to_s, tag_value.to_s)
      end
    end

    def stop
      Rust.drop_agent
    end

    def shutdown
      stop
    end

    private

    def tags_to_string(tags)
      tags.map { |k, v| "#{k}=#{v}" }.join(',')
    end

    def http_headers_to_json(http_headers)
      JSON.dump(http_headers)
    end

  end
end
