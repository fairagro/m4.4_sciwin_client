#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

inputs: []
outputs: 
    - id: array
      type: File[]
      outputBinding:
        glob: "*.txt"
baseCommand: [touch, "array_1.txt", "array_2.txt", "array_3.txt"]