# Pyroscope Python Integration

### What is Pyroscope
[Pyroscope](https://github.com/pyroscope-io/pyroscope) is a tool that lets you continuously profile your applications to prevent and debug performance issues in your code. It consists of a low-overhead agent which sends data to the Pyroscope server which includes a custom-built storage engine. This allows for you to store and query any applications profiling data in an extremely efficient and cost effective way.


### How to install Pyroscope for Python Applications
```
pip install pyroscope-io
```

### Basic Usage of Pyroscope
```
import pyroscope_io as pyroscope

pyroscope.configure(
  application_name       = "my.python.app", # replace this with some name for your application
  server_address         = "http://my-pyroscope-server:4040", # replace this with the address of your pyroscope server
)
```

### Adding Tags
Tags allow for users to view their data at different levels of granularity depending on what "slices" make sense for their application. This can be anything from region or microservice to more dynamic tags like controller or api route.

```
import os
import pyroscope

pyroscope.configure(
  application_name       = "simple.python.app",
  server_address = "http://my-pyroscope-server:4040",

  tags = {
    "hostname": os.getenv("HOSTNAME"),
  }
)

# You can use a wrapper:
with pyroscope.tag_wrapper({ "controller": "slow_controller_i_want_to_profile" }):
  slow_code()
```


### Examples
For more examples see [examples/python](https://github.com/pyroscope-io/pyroscope/tree/main/examples/python) in the main repo.
