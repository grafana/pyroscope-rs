#!/usr/bin/env ruby

require "pyroscope"
require "pyroscope/version"

puts Pyroscope::VERSION
puts RUBY_VERSION

Pyroscope.configure do |config|
  config.application_name = "#{ENV["PYROSCOPE_RUN_ID"]}"
  config.server_address = "https://ingest.pyroscope.cloud"
  config.auth_token = ENV["PYROSCOPE_API_TOKEN"]
  config.detect_subprocesses = ENV["PYROSCOPE_DETECT_SUBPROCESSES"] == "1"
  config.oncpu = ENV["PYROSCOPE_ONCPU"] == "1"
  config.log_level = "trace"
  config.report_pid = true
  config.report_thread_id = true
  config.tags = {
    :region => "us-east",
    :detect_subprocesses => ENV["PYROSCOPE_DETECT_SUBPROCESSES"],
    :oncpu => ENV["PYROSCOPE_ONCPU"],
    :version => ENV["RUBY_VERSION"],
    :arch => ENV["PYROSCOPE_ARCH"]
  }
end

def work(n)
  i = 0
  while i < n
    i += 1
  end
end

def fast_function
  Pyroscope.tag_wrapper({"function": "fast"}) do
    work(2001002000)
  end
end

def slow_function
  work(8001008000)
end

child_pid = fork do
  puts "This is the child process"
  Pyroscope.tag_wrapper({"fork": "forked"}) do
    slow_function()
  end
end

puts "This is the master process."

Pyroscope.tag_wrapper({"fork": "master"}) do
  fast_function()
end

puts "The PID of the child process is #{child_pid}"

Pyroscope.shutdown()
