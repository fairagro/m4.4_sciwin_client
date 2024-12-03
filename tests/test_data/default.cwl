#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

inputs:
- id: file1
  type: File
  default:
    class: File
    path: file.txt

stdout: file.wtf

outputs: 
  out: 
    type: File
    outputBinding:
      glob: 
        file.wtf
arguments:
- cat
- $(inputs.file1.path)
