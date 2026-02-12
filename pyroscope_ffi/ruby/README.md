# Pyroscope Ruby Gem

**Pyroscope integration for Ruby**

[![license](https://img.shields.io/badge/license-Apache2.0-blue.svg)](LICENSE) 
![tests](https://github.com/pyroscope-io/pyroscope-rs/workflows/Tests/badge.svg)
![build](https://github.com/pyroscope-io/pyroscope-rs/workflows/Build/badge.svg)
[![Gem version](https://badge.fury.io/rb/pyroscope.svg)](https://badge.fury.io/rb/pyroscope)

---

### What is Pyroscope
[Pyroscope](https://github.com/pyroscope-io/pyroscope) is a tool that lets you continuously profile your applications to prevent and debug performance issues in your code. It consists of a low-overhead agent which sends data to the Pyroscope server which includes a custom-built storage engine. This allows for you to store and query any applications profiling data in an extremely efficient and cost effective way.


### Supported platforms

| Linux | macOS | Windows | Docker |
|:-----:|:-----:|:-------:|:------:|
|   ✅  |   ✅  |         |   ✅   |

### Profiling Ruby applications

Add the `pyroscope` gem to your Gemfile:

```bash
bundle add pyroscope
```

### Basic Configuration

Add the following code to your application. If you're using rails, put this into `config/initializers` directory. This code will initialize pyroscope profiler and start profiling:

```ruby
require 'pyroscope'

Pyroscope.configure do |config|
  config.application_name = "my.ruby.app" # replace this with some name for your application
  config.server_address   = "http://my-pyroscope-server:4040" # replace this with the address of your pyroscope server
end
```

### Tags

Pyroscope ruby integration provides a number of ways to tag profiling data. For example, you can provide tags when you're initializing the profiler:

```ruby
require 'pyroscope'

Pyroscope.configure do |config|
  config.application_name = "my.ruby.app"
  config.server_address   = "http://my-pyroscope-server:4040"

  config.tags = {
    "hostname" => ENV["HOSTNAME"],
  }
end
```

or you can dynamically tag certain parts of your code:

```ruby
Pyroscope.tag_wrapper({ "controller": "slow_controller_i_want_to_profile" }) do
  slow_code
end
```

### Example

Check out this [example ruby project in our repository](https://github.com/pyroscope-io/pyroscope/tree/main/examples/ruby) for examples of how you can use these features.
