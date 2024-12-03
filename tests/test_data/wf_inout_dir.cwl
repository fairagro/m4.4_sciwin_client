#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: Workflow

inputs:
  dir:
    type: Directory
    default:
      class: Directory
      location: test_dir

outputs:
  newdir:
    type: Directory
    outputSource: dir

steps: []
