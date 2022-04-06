module Pyroscope
  Config = Struct.new(:application_name, :server_address, :sample_rate, :detect_subprocesses, :log_level, :tags)
  class << self
    def configure
      @config = Config.new
      puts @config
    end
  end
end
