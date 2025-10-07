#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: Workflow

inputs:
  first:
    type: string
    default: me

outputs:
  last:
    type: string
    outputSource: first

steps: []
