local build_image = import '../util/build_image.jsonnet';
local pipelines = import '../util/pipelines.jsonnet';

[
  pipelines.linux_amd64('[amd64] make cli/test') {
    trigger: {
      event: ['pull_request'],
    },
    steps: [
      {
        name: 'submodules',
        image: 'alpine/git',
        commands: ['git submodule update --init --recursive'],
      },
      {
        name: 'make cli/test',
        image: build_image.linux,
        commands: ['make cli/test'],
      },
    ],
  },
  pipelines.linux_arm64('[arm64] make cli/test') {
    trigger: {
      event: ['pull_request'],
    },
    steps: [
      {
        name: 'submodules',
        image: 'alpine/git',
        commands: ['git submodule update --init --recursive'],
      },
      {
        name: 'make cli/test',
        image: build_image.linux,
        commands: ['make cli/test'],
      },
    ],
  },
]
