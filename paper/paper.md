---
title: 'SciWin-Client: something subtitle'

tags:
  - CWL
  - workflow
  - FAIRagro

authors:
  - name: Jens Krumsieck
    orcid: 0000-0001-6242-5846
    affiliation: 1
  - name: Antonia Leidel
    orcid: 0009-0007-1765-0527
    affiliation: 2
  - name: Harald von Waldow
    orcid: 0000-0003-4800-2833
    affiliation: 1
  - name: Patrick König
    orcid: 0000-0002-8948-6793
    affiliation: 3
  - name: Xaver Stiensmeier
    orcid: 0009-0005-3274-122X
    affiliation: 4
  - name: Florian Hoedt
    orcid: 0000-0002-6068-1659
    affiliation: 5

affiliations:
  - name: Johann Heinrich von Thünen-Institute, Braunschweig, Germany
    index: 1
  - name:  Leibniz Institute of Plant Genetics and Crop Plant Research, Gatersleben, Germany
    index: 2
  - name: Helmholtz Centre for Infection Research, Braunschweig, Germany
    index: 3
  - name: University Bielefeld, Bielefeld, Germany
    index: 4
  - name: PowerCo ??
    index: 5

date: XX XXXX 202X
bibliography: paper.bib
---

# Summary
SciWIn-Client is a command-line tool developed as part of FAIRagro's Scientific Workflow Infrastructure (SciWIn). It is designed to streamline the creation, execution and management of reproducible computational Workflows using the Common Workflow Language (CWL).
SciWIn-Client allows researchers to create and modify CWL CommandLineTools and workflows with minimal amount of work. It supports local and remote workflow execution within containerized environments enabling consistent results across different systems.
By integrating with Git it promotes transparent and collaborative workflow development helping researchers to share their work in a reproducible and substainable way.
...

# Statement of Need
Automated computational workflows have become essential across various scientific disciplines to manage complex, multi-step data analysis procedures.[ref?] Significant resources have been invested into domain specific languages that formalize and standardize computational scientific processes, thereby enhancing reproducibility, scalability and efficiency. In order to harmonize and standardize the proliferation of workflow languages, the Common Workflow Language (CWL)[@Crusoe2022MethodsIncluded] was created as a higher-level standard. However, CWL's verbose syntax, while enabling great flexibility, creates a steep learning curve that may hinder the language from reaching its full potential in the broader scientific community. On the other hand, CWL's foundation in the YAML file format ensures excellent machine readability and actionability, facilitating automated processing and integration. CWL is actually predestined to be written by machines rather than humans, which ultimately led to the conception of SciWIn-Client.

SciWIn-Client is a command-line application based on Rust that makes it simple to transform scripts and command-line tools into complex CWL Workflows...
... FAIRagro-Consortium[@Ewert2023Proposal]

# Features and Implementation

![Overview of ...](assets/overview.png)

...


## Creation of CWL Files
To create CWL CommandLineTools, users must prefix their tool or script invocations with the "s4n create" prefix. The SciWIn client automatically identifies the inputs of the CommandLineTool based on the program call. Outputs are the newly generated files produced after executing a command or script. When a new project is initialized, a Git repository is created in the background to provide a version-controlled environment for tracking changes and support this process. While the system can automatically infer inputs and outputs, users also have the option to define them explicitly. Users can specify a container image pulled from Docker Hub or provide paths to local Dockerfiles to ensure consistent, reproducible execution environments across different systems.

## Workflow Execution
For the execution on remote compute clusters SciWIn-Client is able to communicate with the REST-API of Reana[@Simko2019Reana]. Structured execution results in form of RO-crates[@SoilandReyes2022ROCrate] more specifically Workflow Run RO-Crates[@Leo2024WRRC] using the Provenance Run Crate profile can be exported. ...
...
## ...
...

# Acknowledgements 
We gratefully acknowledge the financial support of the German Research Foundation (DFG) – project number 501899475.

# References