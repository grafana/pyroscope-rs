{
  local version = std.extVar('BUILD_IMAGE_VERSION'),

  linux: 'grafana/agent-build-image:%s' % version,

}
