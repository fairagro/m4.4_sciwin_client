# SciWIn Client - Scientific Workflow Infrastructure
Reproducibility in computational research is vital for efficient collaboration, verifying results and ensuring transparency. Yet it remains challenging due to complex workflows, inconsistent data management and the reliance on specific software environments. SciWIn Client is a command-line tool designed to easily create, record, annotate and execute computational workflows. SciWIn Client enables researchers to interactively use intuitive commands to keep track of tasks such as as data-extraction, -cleaning, -transformation, -analysis, -visualization and computational simulation. Automated and standardised workflows minimise sources of error and support transparent and reproducible Open Science.

# Usage
## Project initialization
Most commands need the context of a Git repo to work. Project initialization can be done using the `s4n init` command.
```bash
s4n init -p <FOLDER/PROJECT NAME>
```
Besides the minimal project structure, the creation of an ["Annotated Research Context"](https://arc-rdm.org/) or ARC is also possible.
```bash
s4n init -a -p <FOLDER/PROJECT NAME>
```

## Creation of CWL Files
To create [CWL](https://www.commonwl.org/) CommandLineTools which can be combined to workflows later a prefix command can be used. `s4n tool create` which has `s4n run` as a synonym will execute any given command and creates a CWL CommandLineTool accordingly.
```bash
s4n tool create <COMMAND> [ARGUMENTS]
```

# Build
![Rust][rust-image]
![Coverage][coverage-badge]

This project is being developed using Rust and Cargo. To run the source code use `cargo run`, to build use `cargo build`.

# Testing
To run the tests use `cargo test` or `cargo test -- --nocapture` to output logs.

<!--section images-->
[coverage-badge]: https://coverage.jenskrumsieck.de/coverage/fairagro/m4.4_sciwin_client
[rust-image]: https://img.shields.io/badge/Rust-%23000000.svg?e&logo=rust&logoColor=white
