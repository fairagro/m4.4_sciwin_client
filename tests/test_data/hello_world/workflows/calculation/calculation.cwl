#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

requirements:
- class: InitialWorkDirRequirement
  listing:
  - entryname: workflows/calculation/calculation.py
    entry:
      $include: calculation.py

inputs:
- id: population
  type: File
  default:
    class: File
    location: '../../data/population.csv'
  inputBinding:
    prefix: '--population'
- id: speakers
  type: File
  default:
    class: File
    location: '../../data/speakers_revised.csv'
  inputBinding:
    prefix: '--speakers'

outputs:
- id: results
  type: File
  outputBinding:
    glob: results.csv

baseCommand:
- python
- workflows/calculation/calculation.py
