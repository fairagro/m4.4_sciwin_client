#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: Workflow

inputs:
- id: pop
  type: File

outputs:
- id: out
  type: File
  outputSource: echo/results

steps:
- id: echo
  in:
    test: pop
  run: echo.cwl
  out:
  - results
