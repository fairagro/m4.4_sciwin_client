# Tool Creation

CWL command line tools can be created easily using `s4n`. The simplest approach is to just add the `s4n tool create` or `s4n run` as prefix to the command.

A command line tool consists of a `baseCommand` which is usually any kind of executable which accepts `inputs` and writes `outputs`. In- and Outputs can be of multiple kinds of value type but Files are generally the most used kind. The `baseCommand` can be a single term like `echo` or a list like `[python, script.py]`. All of this is handle by SciWIn Client.

!!! note 
    The following examples assume that they are being executed in a git repository with clean status. If there is no repository yet, use `s4n init` to create an environment.

## Wrapping `echo`
A common example is to wrap the `echo` command for its simplicity. To create the tool `echo "Hello World"` is prefixed with `s4n tool create`.
=== ":octicons-terminal-16: Command"
    ```
    s4n tool create echo "Hello World"
    ```
=== ":simple-commonworkflowlanguage: echo.cwl"
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: CommandLineTool

    inputs:
    - id: hello_world
      type: string
      default: Hello World
      inputBinding:
        position: 0

    outputs: []
    baseCommand: echo
    ```

The baseCommand was correctly determined as `echo` and an input slot was created named with the value of the input as slug `hello_world`. This could be renamed by editing the file, but for now we leave it as is. Currrently the tool is not producing any outputs. Assuming we want to create a file using the `echo` command which is a common use case a redirection `>` operator can be used. To not redirect the `s4n` output it needs to be shielded by a backslash `\>`.
Assuming the following `yaml` file needs to be created 
```yaml
message: "Hello World"
```
The usual command would be `echo 'message: "Hello World"' > hello.yaml`. To create the command line tool the command will be
=== ":octicons-terminal-16: Command"
    ```
    s4n tool create --name echo2 echo 'message: "Hello World"' \> hello.yaml
    ```
=== ":simple-commonworkflowlanguage: echo2.cwl"
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: CommandLineTool

    inputs:
    - id: message_hello_world
      type: string
      default:
        message: Hello World
      inputBinding:
        position: 0

    outputs:
    - id: hello
      type: File
      outputBinding:
        glob: hello.yaml
    stdout: hello.yaml

    baseCommand: echo
    ```

The `stdout` part will tell the tool to redirect output to the file called `hello.yaml`. Noticed the `--name` option? This is used to specify the file name of the to be created tool.

## Wrapping a python script
A common usecase is to wrap a script in an interpreted language like python or R. Wrapping a python script follows the same principles like shown in the previous example where the `echo` command was wrapped.

=== ":octicons-terminal-16: Command"
    ```
    s4n tool create --name echo_python python echo.py --message "SciWIn rocks!" --output-file out.txt
    ```
=== ":simple-python: echo.py"
    ```python
    import argparse;

    parser = argparse.ArgumentParser(description='Echo your input')
    parser.add_argument('--message', help='Message to echo', required=True)
    parser.add_argument('--output-file', help='File to save the message', required=True)

    args = parser.parse_args()

    with open(args.output_file, 'w') as f:
        f.write(args.message)
        print(args.message)
    ``` 
=== ":simple-commonworkflowlanguage: echo_python.cwl"
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: CommandLineTool

    requirements:
    - class: InitialWorkDirRequirement
      listing:
      - entryname: echo.py
        entry:
          $include: '../../echo.py'
      - class: InlineJavascriptRequirement

    inputs:
    - id: message
      type: string
      default: SciWIn rocks!
      inputBinding:
        prefix: '--message'
    - id: outputfile
      type: string
      default: out.txt
      inputBinding:
        prefix: '--output-file'

    outputs:
    - id: out
      type: File
      outputBinding:
        glob: $(inputs.outputfile)

    baseCommand:
    - python
    - echo.py
    ```

As shown in `echo_python.cwl` the `outputBinding` for the output file is set to `$(inputs.outputfile)` and therefore automatically gets the name given in the input. `s4n` also automatically detects the usage of `python` and adds the used script as an `InitialWorkDirRequirement` which makes the script available for the execution engine.

## Wrapping a long running script
Sometimes it is neccessary to run a highly complicated script on a remote environment because it would take to long on a simple machine. But how to get the CWL file than? In the example python file the script will sleep for 1 minute and then writes a file. One could use the `s4n tool create` command as shown above and just wait 60 seconds. But what if the calculation takes a week? This is possible for example in quantum chemical calculations like DFT.
There is the `--no-run` flag which tells `s4n` to not run the script. However this will not create an output and therefore can not detect any output files.

=== ":octicons-terminal-16: Command"
    ```
    s4n tool create --no-run python sleep.py
    ```
=== ":simple-python: sleep.py"
    ```python
    from time import sleep

    sleep(60)

    with open('sleep.txt', 'w') as f:
        f.write('I slept for 60 seconds')
    ```
=== ":simple-commonworkflowlanguage: sleep.cwl"
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: CommandLineTool

    requirements:
    - class: InitialWorkDirRequirement
      listing:
      - entryname: sleep.py
        entry:
          $include: '../../sleep.py'

    inputs: []
    outputs: []
    baseCommand:
    - python
    - sleep.py
    ```

For this cases there is the possibility to specify outputs via the commandline using the `-o` or `--outputs` argument which tells the parser to add a output slot.
=== ":octicons-terminal-16: Command"
    ```
    s4n tool create --name sleep2 --no-run -o sleep.txt python sleep.py
    ```
=== ":simple-commonworkflowlanguage: sleep2.cwl"
    ```yaml
    #!/usr/bin/env cwl-runner
    
    cwlVersion: v1.2
    class: CommandLineTool
    
    requirements:
    - class: InitialWorkDirRequirement
      listing:
      - entryname: sleep.py
        entry:
          $include: '../../sleep.py'
    
    inputs: []
    outputs:
    - id: sleep
      type: File
      outputBinding:
        glob: sleep.txt
    
    baseCommand:
    - python
    - sleep.py
    ```

This CWL file can then be executed remotely by using any runner e.g. `cwltool` and will write the `sleep.txt` file after 60 seconds.