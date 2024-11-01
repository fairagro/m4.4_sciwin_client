#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

inputs:
- id: la
  type: boolean
  default: true
  inputBinding:
    prefix: '-la'

outputs: []
baseCommand: ls
