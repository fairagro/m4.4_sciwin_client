# Tool Commands
`s4n`s tool commands are designed to facilitate the interoperability with CWL CommandLineTools. The tool command itself has three subcommands for basic CR~~U~~D operations: `create`, `list` and `remove`.

```
Provides commands to create and work with CWL CommandLineTools

Usage: s4n tool <COMMAND>

Commands:
  create  Runs commandline string and creates a tool (synonym: s4n run)
  list    Lists all tools [aliases: ls]
  remove  Remove a tool, e.g. s4n tool rm toolname [aliases: rm]
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## `tool create`
The `tool create` command can be used to easily generate CWL CommandLineTools. It serves as a prefix to the usual command line prompt. Calling `tool create` with a command attached will execute the command, determine in- and outputs and create a CWL tool definition file in the `workflows` folder. `s4n tool create` has `s4n run` as alias for even less typing. 
> [!NOTE]
> Before using this command all changes need to be commited as it uses `git` to determine tool outputs!

```
Runs commandline string and creates a tool (synonym: s4n run)

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
  -h, --help                               Print help
```

With the `--name` option the resulting filename can be manipulated. Without the argument SciWIn client will automatically generate a name based on the command. If for example the same base command is used in two tools there would be a file name conflict.

The two container options `--container-image` and `--container-tag` can be used to add Docker requirements to the resulting CWL file. However Docker will **NOT** be used to execute the script while generating the tool, so make sure to either use `s4n` in Docker container or provide all neccesary tools for it to run.

With the `--raw` flag no CWL file will be written to disk. Instead the raw cwl will be outputted to the command prompt (stdout) to process it further.

As `tool create` needs all changes to be commited beforehand it will create a commit after it completed the tool creation. To prevent that because e.g. manual changes to the CWL file have to be made before committing the `--no-commit` flag can be used.

Some scripts tend to run for a very long time - e.g. quantum chemistry calculations. To prevent the tool from running the `--no-run` flag can be used. If this flag is set the parser will just use information from the command line to create the tool, outputs need to be set manually.

**TODO: Manual In and Outputs!!!** (See #44)

Sometimes it can be beneficial to not commit the created outputs. With the `--clean` flag all outputs will be deleted before commiting the freshly created tool.

## `tool list`

`tool list` or `tool ls` can be used to list all existing tools. Using the command without the `-a` flag just ouputs the names of all existing tools in the project. Using the `-all` (or `-a`) flag will also output the tools in- and outputs which than can easily be used for the `workflow connect` command.

```
Lists all tools

Usage: s4n tool list [OPTIONS]

Options:
  -a, --all   Outputs the tools with inputs and outputs
  -h, --help  Print help
```

## `tool remove`
The `tool remove` or `tool rm` command can be used to delete one or more CWL CommandLineTools. 

```
Remove a tool, e.g. s4n tool rm toolname

Usage: s4n tool remove [TOOL_NAMES]...

Arguments:
  [TOOL_NAMES]...  Remove a tool

Options:
  -h, --help  Print help
```