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
- id: out
  type: File
  outputSource: plot/o_results
- id: merzlos
  type: Any

steps:
- id: plot
  in:
  - id: results
    source: calculation/results
  run: ../plot/plot.cwl
  out:
  - o_results
- id: calculation
  in:
  - id: population
    source: population
  - id: speakers
    source: speakers
  run: ../calculation/calculation.cwl
  out:
  - results
