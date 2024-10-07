class: CommandLineTool
cwlVersion: v1.2
baseCommand:
- python3
- tests/test_data/echo.py
inputs:
- id: test
  type: File
  inputBinding:
    prefix: '--test'
  default:
    class: File
    location: tests/test_data/input.txt
outputs:
- id: results
  type: File
  outputBinding:
    glob: results.txt
requirements:
- class: InitialWorkDirRequirement
  listing:
  - entryname: tests/test_data/echo.py
    entry:
      $include: tests/test_data/echo.py
- class: DockerRequirement
  dockerFile:
    $include: tests/test_data/Dockerfile
  dockerImageId: sciwin-container
