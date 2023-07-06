{
  local version = std.extVar('BUILD_IMAGE_VERSION'),

  linux: 'pyroscope/rust_builder_cli:%s' % version,

}
