# Workflow commands
The workflow commands provide easy ways to perform operations on CWL Workflow files. This features basic CR~~U~~D operations and the possibility to connect and disconnect steps.

```
Provides commands to create and work with CWL Workflows

Usage: s4n workflow <COMMAND>

Commands:
  create      Creates a blank workflow
  connect     Connects a workflow node
  disconnect  Disconnects a workflow node
  save        Saves a workflow
  status      Shows socket status of workflow
  list        List all workflows [aliases: ls]
  remove      Remove a workflow [aliases: rm]
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## `workflow create`
The `s4n workflow create` command creates an empty CWL workflow definition with the given name. An exisiting workflow can be overwritten using the `--force` flag.

```
Creates a blank workflow

Usage: s4n workflow create [OPTIONS] <NAME>

Arguments:
  <NAME>  A name to be used for this tool

Options:
  -f, --force  Overwrites existing workflow
  -h, --help   Print help
```

!!! example
    ```
    s4n workflow create my-workflow
    ```
    will create an empty workflow file in the workflows folder.
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: Workflow

    inputs: []
    outputs: []
    steps: []
    ```

## `workflow connect` and `workflow disconnect`
The `workflow connect` and `workflow disconnect` commands can be used to connect CWL CommandLineTools, workflow inputs and workflow outputs forming a directed acyclic graph. The connect command establishes a connection and adds in-, outputs and steps if they are not present in the current workflow. The names of the steps slots can be copied from the output of `s4n tool ls`. For connections to in- or outputs a `@` has to be used es prefix e.g. `@inputs/my-file`. The name of the node is constructed by using the tool's name and the name of the tool's node separated by a forward slash: `mytool/my-input`. Connections are made using the `--from` and `--to` arguments together with the name of the workflow.

```
Connects a workflow node

Usage: s4n workflow connect --from <FROM> --to <TO> <NAME>

Arguments:
  <NAME>  Name of the workflow name to be altered

Options:
  -f, --from <FROM>  Starting Node: [tool]/[output]
  -t, --to <TO>      Ending Node: [tool]/[input]
  -h, --help         Print help
```

!!! example "Connect to input socket"
    ```
    s4n workflow connect my-workflow --from @inputs/population --to calculation/population
    ```
    Will add a new input socket `population` and adds the `calculation` step if it is not present in the workflow already. The step's input `population` will be mapped to the workflow's input.
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: Workflow

    inputs:
    - id: population
      type: File

    outputs: []
    steps:
    - id: calculation
      in:
        population: population
      run: '../calculation/calculation.cwl'
      out:
      - results
    ```
    Connecting to workflow outputs works analogues.

!!! example "Connecting Tools"
    ```
    s4n workflow connect my-workflow --from calculation/results --to plot/results
    ```
    Will add a connection between two CWL CommandLineTools mapping the `results` output of the `calculation` tool to the `results` input of the `plot` tool.
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: Workflow

    inputs: []
    outputs: []
    steps:
    - id: calculation
      in: {}
      run: '../calculation/calculation.cwl'
      out:
      - results
    - id: plot
      in:
        results: calculation/results
      run: '../plot/plot.cwl'
      out:
      - results
    ```

The same logic applies for the disconnect command.
```
Disconnects a workflow node

Usage: s4n workflow disconnect --from <FROM> --to <TO> <NAME>

Arguments:
  <NAME>  Name of the workflow name to be altered

Options:
  -f, --from <FROM>  Starting Node: [tool]/[output]
  -t, --to <TO>      Ending Node: [tool]/[input]
  -h, --help         Print help
```

## `workflow save`
The save command simply commits the changes made to a workflow using git.

```
Saves a workflow

Usage: s4n workflow save [OPTIONS] <NAME>

Arguments:
  <NAME>  A name to be used for this tool

Options:
  -f, --force  Overwrites existing workflow
  -h, --help   Print help
```

## `workflow status`
The `workflow status` command shows the current connection status of a workflow. Successfully connected sockets are marked in green, a gray icon shows the usage of a tool's default value and the red cross shows unconnected sockets.

```bash
s4n workflow status main
# Status report for Workflow workflows/main/main.cwl
# +--------------------------------+------------------+---------------+
# | Tool                           | Inputs           | Outputs       |
# +================================+==================+===============+
# | <Workflow>                     | ✅    speakers   |               |
# +--------------------------------+------------------+---------------+
# | Steps:                         |                  |               |
# +--------------------------------+------------------+---------------+
# | ../calculation/calculation.cwl | ✅    speakers   | ❌    results |
# |                                | 🔘    population |               |
# +--------------------------------+------------------+---------------+
# ✅ : connected - 🔘 : tool default - ❌ : no connection
```

```
Shows socket status of workflow

Usage: s4n workflow status [OPTIONS] <NAME>

Arguments:
  <NAME>  A name to be used for this tool

Options:
  -f, --force  Overwrites existing workflow
  -h, --help   Print help
```

## `workflow list`
`s4n workflow list` lists all workflows of the current project. Using the `--all` flag more information about steps and in- and outputs can be shown.
```
List all workflows

Usage: s4n workflow list [OPTIONS]

Options:
  -a, --all   Outputs the tools with inputs and outputs
  -h, --help  Print help
```

## `workflow remove`
`s4n workflow remove` can be used to delete a workflow from the project.

```
Remove a workflow

Usage: s4n workflow remove [RM_WORKFLOW]...

Arguments:
  [RM_WORKFLOW]...  Remove a workflow

Options:
  -h, --help  Print help
```