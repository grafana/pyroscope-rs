{
  linux(name):: {
    kind: 'pipeline',
    type: 'docker',
    name: name,
    platform: {
      os: 'linux',
      arch: 'amd64',
    },
  },
}
