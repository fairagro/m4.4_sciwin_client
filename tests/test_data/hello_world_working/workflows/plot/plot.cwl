cwlVersion: v1.2
class: CommandLineTool

requirements:
- class: InitialWorkDirRequirement
  listing:
  - entryname: plot.py
    entry:
      $include: plot.py
- class: DockerRequirement
  dockerPull: user12398/pytest:v1.0.0

inputs:
- id: results
  type: File
  default:
    class: File
    location: ../../results.csv
  inputBinding:
    prefix: '--results'

outputs:
- id: outputs
  type: File
  outputBinding:
    glob: results.svg

baseCommand:
- python
- plot.py
