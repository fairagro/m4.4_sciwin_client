#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: Workflow

inputs:
- id: population
  type: File
- id: speakers
  type: File

outputs:
- id: out
  type: File
  outputSource: plot/results

steps:
- id: calculation
  in:
    population: population
    speakers: speakers
  run: '../calculation/calculation.cwl'
  out:
  - results
- id: plot
  in:
    results: calculation/results
  run: '../plot/plot.cwl'
  out:
  - results