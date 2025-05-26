# Workflow Creation
Workflows are a key part of SciWIn as they are even part of the name (Scientific Workflow Infrastructure). SciWin Client facilitates the creation of CWL workflows from exisiting CWL CommandLineTools. Workflows in `s4n` are created piece by piece using a command for each edge of the directed acyclic graph (DAG) that represents the workflow.

![Workflow](../assets/simple_workflow.svg)
/// caption
Simple Workflow 
///

## Creating a blank workflow
A blank workflow file can be created by using the `s4n workflow create` command.
=== ":octicons-terminal-16: Command"
    ```
    s4n workflow create my-workflow
    ```
=== ":simple-commonworkflowlanguage: my-workflow.cwl"
    ```yaml
    #!/usr/bin/env cwl-runner
    
    cwlVersion: v1.2
    class: Workflow
    
    inputs: []
    outputs: []
    steps: []
    ```

## Creating connections
Connections can be seen as the arrows in the above figure. Think about a sentence like 
> The arrow points from the input slot called `speakers` to the calculation steps input.

Therefore the command that created this arrow was `s4n workflow connect my-workflow --from @inputs/speakers --to calculation/speakers`.

In this simple example we will connect the `echo` command with the `cat` command. Use the following Commands to create the needed command line tool specifications.
```
s4n tool create echo "Hello World" \> greeting.txt
s4n tool create cat greeting.txt 
s4n workflow create echo-cat
```

To get an overview of the available slots the `s4n tool ls -a` command can be used.
```
+------+------------------+---------------+
| Tool | Inputs           | Outputs       |
+------+------------------+---------------+
| echo | echo/hello_world | echo/greeting |
+------+------------------+---------------+
| cat  | cat/greeting_txt |               |
+------+------------------+---------------+
```

As input parameter we wish to use the `hello-world` input of the `echo` tool and connect the output `echo/greeting` to the input `cat/greeting_txt`. Therefore 3 connections are needed.

### Connecting a new input to echo step
=== ":octicons-terminal-16: Command"
    ```
    s4n workflow connect echo-cat --from @inputs/message --to echo/hello_world
    ```
=== ":simple-commonworkflowlanguage: echo-cat.cwl"
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: Workflow

    inputs:
    - id: message
      type: string

    outputs: []
    steps:
    - id: echo
      in:
        hello_world: message
      run: '../echo/echo.cwl'
      out:
      - greeting

    ```

### Connecting the output of the echo step to the cat step
=== ":octicons-terminal-16: Command"
    ```
    s4n workflow connect echo-cat --from echo/greeting --to cat/greeting_txt
    ```
=== ":simple-commonworkflowlanguage: echo-cat.cwl"
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: Workflow

    inputs:
    - id: message
      type: string

    outputs: []
    steps:
    - id: echo
      in:
        hello_world: message
      run: '../echo/echo.cwl'
      out:
      - greeting
    - id: cat
      in:
        greeting_txt: echo/greeting
      run: '../cat/cat.cwl'
      out: []
    ```

To save the workflow `s4n workflow save echo-cat` is used.

Workflow visualizations can be achieved using `s4n workflow visualize`
```
s4n workflow visualize -r dot workflows/echo-cat/echo-cat.cwl | dot -Tsvg > workflow.svg
```

![created workflow](../assets/workflow_01.svg)
/// caption
The created `echo-cat` workflow as DAG representation.
///