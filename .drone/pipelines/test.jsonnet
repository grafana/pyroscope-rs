local build_image = import '../util/build_image.jsonnet';
local pipelines = import '../util/pipelines.jsonnet';

[
  // todo add macos builds
  pipelines.linux_amd64('[amd64] make test') {
    trigger: {
      event: ['pull_request'],
    },
    steps: [
      {
        name: 'make cli/test',
        image: build_image.linux,
        commands: ['uname -a', 'make test'],
      },
    ],
  },
  pipelines.linux_arm64('[arm64] make test') {
    trigger: {
      event: ['pull_request'],
    },
    steps: [
      {
        name: 'make cli/test',
        image: build_image.linux,
        commands: ['uname -a', 'make test'],
      },
    ],
  },
]
