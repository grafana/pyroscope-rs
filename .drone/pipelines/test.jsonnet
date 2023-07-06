local build_image = import '../util/build_image.jsonnet';
local pipelines = import '../util/pipelines.jsonnet';

[
  pipelines.linux('make cli/test') {
    trigger: {
      event: ['pull_request'],
    },
    steps: [{
      name: 'make cli/test',
      image: build_image.linux,
      commands: [
        'make cli/test',
      ],
    }],
  },
]
