# Pyroscope Python Package

**Pyroscope integration for Python**

[![license](https://img.shields.io/badge/license-Apache2.0-blue.svg)](LICENSE) 
![tests](https://github.com/pyroscope-io/pyroscope-rs/workflows/Tests/badge.svg)
![build](https://github.com/pyroscope-io/pyroscope-rs/workflows/Build/badge.svg)
[![PyPI version](https://badge.fury.io/py/pyroscope-io.svg)](https://badge.fury.io/py/pyroscope-io)
[![PyPI](https://img.shields.io/pypi/pyversions/pyroscope-io.svg?maxAge=2592000)](https://pypi.python.org/pypi/pyroscope-io)


---

### What is Pyroscope
[Pyroscope](https://github.com/pyroscope-io/pyroscope) is a tool that lets you continuously profile your applications to prevent and debug performance issues in your code. It consists of a low-overhead agent which sends data to the Pyroscope server which includes a custom-built storage engine. This allows for you to store and query any applications profiling data in an extremely efficient and cost effective way.


### How to install Pyroscope for Python Applications
```
pip install pyroscope-io
```

### Minimal Configuration

Add the following code to your application. This code will initialize pyroscope profiler and start profiling:

```python
import pyroscope

pyroscope.configure(
  application_name = "my.python.app", # replace this with some name for your application
  server_address   = "http://my-pyroscope-server:4040", # replace this with the address of your pyroscope server
)
```

### Full Configuration

Optionally, you can configure several parameters:

```python
import pyroscope

pyroscope.configure(
  application_name    = "my.python.app", # replace this with some name for your application
  server_address      = "http://my-pyroscope-server:4040", # replace this with the address of your pyroscope server
  auth_token          = "{YOUR_API_KEY}", # optional, if authentication is enabled, specify the API key
  sample_rate         = 100, # default is 100
  detect_subprocesses = False, # detect subprocesses started by the main process; default is False
  oncpu               = True # report cpu time only; default is True
  gil_only            = True # only include traces for threads that are holding on to the Global Interpreter Lock; default is True
  log_level           = "info" # default is info, possible values: trace, debug, info, warn, error and critical 
  tags           = {
    "region":   '{os.getenv("REGION")}',
  }
)

```

### Tags

You can add tags to certain parts of your code:

```python
# You can use a wrapper:
with pyroscope.tag_wrapper({ "controller": "slow_controller_i_want_to_profile" }):
  slow_code()
```

### Span profiles support for OpenTelemetry
Register the `PyroscopeSpanProcessor` in your `OpenTelemetry` integration:
```python3
# import span processor
from pyroscope.otel import PyroscopeSpanProcessor

# obtain a OpenTelemetry tracer provider
from opentelemetry import trace
from opentelemetry.sdk.trace import TracerProvider
provider = TracerProvider()

# register the span processor
provider.add_span_processor(PyroscopeSpanProcessor())

# register the tracer provider
trace.set_tracer_provider(provider)

# ...
```

### Example

Check out this [example python project in our repository](https://github.com/pyroscope-io/pyroscope/tree/main/examples/python) for examples of how you can use these features.
