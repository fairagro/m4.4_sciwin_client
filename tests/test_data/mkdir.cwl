#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

inputs:
- id: dirname
  type: string

outputs: 
  out: 
    type: Directory
    outputBinding:
      glob: 
        $(inputs.dirname)
arguments:
- mkdir
- $(inputs.dirname)
