#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

requirements:
- class: InitialWorkDirRequirement
  listing:
  - entryname: workflows/plot/plot.py
    entry:
      $include: plot.py

inputs:
- id: results
  type: File
  default:
    class: File
    location: '../../results.csv'
  inputBinding:
    prefix: '--results'

outputs:
- id: results
  type: File
  outputBinding:
    glob: results.svg

baseCommand:
- python
- workflows/plot/plot.py
