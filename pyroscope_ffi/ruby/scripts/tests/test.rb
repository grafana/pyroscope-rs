#!/usr/bin/env ruby

require "pyroscope"
require "pyroscope/version"

puts Pyroscope::VERSION
puts RUBY_VERSION

def start_local_pyroscope
  container_name = "pyroscope-ruby-test-#{Process.pid}"
  system(
    "docker", "run", "-d",
    "--name", container_name,
    "-p", "4040:4040",
    "grafana/pyroscope:latest",
    "-ingester.min-ready-duration=0s"
  )

  unless $?.success?
    warn "failed to start local grafana/pyroscope container"
    exit 1
  end

  20.times do
    ready = system("curl", "-fsS", "http://localhost:4040/ready")
    return container_name if ready
    sleep 1
  end

  warn "pyroscope container did not become ready"
  warn "==== pyroscope container status ===="
  system("docker", "ps", "-a", "--filter", "name=#{container_name}")
  warn "==== pyroscope container logs ===="
  system("docker", "logs", container_name)
  system("docker", "rm", "-f", container_name)
  exit 1
end

def stop_local_pyroscope(container_name)
  return if container_name.nil? || container_name.empty?

  system("docker", "rm", "-f", container_name)
end

pyroscope_container = start_local_pyroscope
at_exit { stop_local_pyroscope(pyroscope_container) }

Pyroscope.configure do |config|
  config.application_name = "#{ENV["PYROSCOPE_RUN_ID"]}"
  config.server_address = "http://localhost:4040"
  config.oncpu = ENV["PYROSCOPE_ONCPU"] == "1"
  config.log_level = "trace"
  config.report_pid = true
  config.report_thread_id = true
  config.tags = {
    :region => "us-east",
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
