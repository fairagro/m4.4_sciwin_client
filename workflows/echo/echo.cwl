#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

requirements:
- class: InitialWorkDirRequirement
  listing:
  - entryname: './tests/test_data/echo.py'
    entry:
      $include: '../.././tests/test_data/echo.py'

inputs:
- id: tes
  type: File
  default:
    class: File
    location: '../.././tests/test_data/input.txt'
  inputBinding:
    prefix: '--tes'

outputs:
- id: results
  type: File
  outputBinding:
    glob: results.txt

baseCommand:
- python
- './tests/test_data/echo.py'
