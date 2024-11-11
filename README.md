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

## Creation of CWL CommandLineTools
To create [CWL](https://www.commonwl.org/) CommandLineTools which can be combined to workflows later a prefix command can be used. `s4n tool create` which has `s4n run` as a synonym will execute any given command and creates a CWL CommandLineTool accordingly.
```bash
s4n tool create <COMMAND> [ARGUMENTS]
```
The command comes with a lot of different options on how to handle the CWL creation specifically.
```
Usage: s4n tool create [OPTIONS] [COMMAND]...

Arguments:
  [COMMAND]...  Command line call e.g. python script.py [ARGUMENTS]

Options:
  -n, --name <NAME>                        A name to be used for this tool
  -c, --container-image <CONTAINER_IMAGE>  An image to pull from e.g. docker hub or path to a Dockerfile
  -t, --container-tag <CONTAINER_TAG>      The tag for the container when using a Dockerfile
  -r, --raw                                Outputs the raw CWL contents to terminal
      --no-commit                          Do not commit at the end of tool creation
      --no-run                             Do not run given command
      --clean                              Deletes created outputs after usage
```

## Creation of Workflows
CWL Workflows can be created semi-automatically using `s4n workflow` commands. First of all a workflow needs to be created.
```bash
s4n workflow create <NAME>
```
After execution of this command a file called `workflows/<NAME>/<NAME>.cwl` will be created. 
Workflow Steps and Connections can be added using the `s4n workflow connect` command. Connections to In- or Outputs are added using either `@inputs` or `@outputs` as file identifier.
```bash
s4n workflow connect <NAME> --from [FILE]/[SLOT] --to [FILE/SLOT]
```
For example: `s4n workflow connect demo --from @inputs/speakers --to calculation/speakers` - The Step `calculation` will be added pointing to `workflows/calculation/calculation.cwl`, which will use the newly created input `speakers` as input for its `speakers` input.

## Execution of CWL Files
SciWIn-Client comes with its custom CWL Runner (which does not support all `cwltool` can do, yet!) to run the CommandLineTools (Workflows to be added soon!). The command `s4n execute local` can also be triggered using `s4n ex l`.
```bash
s4n execute local <CWLFILE> [ARGUMENTS]
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
