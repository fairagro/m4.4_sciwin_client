# Tool Creation

CWL command line tools can be created easily using `s4n`. The simplest approach is to just add the `s4n tool create` or `s4n run` as prefix to the command.

A command line tool consists of a `baseCommand` which is usually any kind of executable which accepts `inputs` and writes `outputs`. In- and Outputs can be of multiple kinds of value type but Files are generally the most used kind. The `baseCommand` can be a single term like `echo` or a list like `[python, script.py]`. All of this is handle by SciWIn Client.

!!! note 
    The following examples assume that they are being executed in a git repository with clean status. If there is no repository yet, use `s4n init` to create an environment.

## Wrapping `echo`
A common example is to wrap the `echo` command for its simplicity. To create the tool `echo "Hello World"` is prefixed with `s4n tool create`.
```
s4n tool create echo "Hello World"
```
The produced `echo.cwl` will look like this:
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
```
s4n tool create --name echo2 echo 'message: "Hello World"' \> hello.yaml
```
The produced `echo2.cwl` will look like this:
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