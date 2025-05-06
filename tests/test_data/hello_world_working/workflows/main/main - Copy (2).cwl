#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: Workflow

inputs:
- id: population
  type: File
  default:
    class: File
    location: ../../data/population.csv
- id: speakers
  type: File
  default:
    class: File
    location: ../../data/speakers_revised.csv


outputs:
- id: outputs
  type: File
  outputSource: plot/outputs

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
  - id: outputs