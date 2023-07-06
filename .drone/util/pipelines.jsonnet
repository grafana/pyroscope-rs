{
  linux_amd64(name):: {
    kind: 'pipeline',
    type: 'docker',
    name: name,
    platform: {
      os: 'linux',
      arch: 'amd64',
    },
  },
  linux_arm64(name):: {
    kind: 'pipeline',
    type: 'docker',
    name: name,
    platform: {
      os: 'linux',
      arch: 'arm64',
    },
  },
}
