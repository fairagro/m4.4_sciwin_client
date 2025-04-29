#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

requirements:
- class: DockerRequirement
  dockerPull: alpine:latest

inputs:
- id: file
  type: File
  default:
    class: File
    location: |-
      https://www.bundeswahlleiterin.de/bundestagswahlen/2025/ergebnisse/opendata/btw25/csv/kerg2.csv
  inputBinding:
    position: 1

outputs: []

baseCommand: echo
