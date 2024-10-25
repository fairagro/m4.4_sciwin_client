#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

requirements:
- class: InitialWorkDirRequirement
  listing:
  - entryname: tests/test_data/echo2.py
    entry:
      $include: '../../tests/test_data/echo2.py'

inputs:
- id: t
  type: File
  default:
    class: File
    location: '../../tests/test_data/input.txt'
  inputBinding:
    prefix: '-t'
- id: t2
  type: File
  default:
    class: File
    location: '../../tests/test_data/input2.txt'
  inputBinding:
    prefix: '-t2'

outputs:
- id: results
  type: File
  outputBinding:
    glob: results.txt
- id: results2
  type: File
  outputBinding:
    glob: results2.txt

baseCommand:
- python
- tests/test_data/echo2.py
