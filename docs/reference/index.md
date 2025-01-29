# Reference
SciWIn client provides commands for project initialization ([`s4n init`](init.md)), working with CWL CommandLineTools ([`s4n tool`](tool.md)) and CWL Workflows ([`s4n workflow`](workflow.md)), metadata annotation ([`s4n annotate`](annotate.md)), the execution of CWL ([`s4n execute`](execute.md)) and synchronization with a remote sever ([`s4n sync`](sync.md)).

!!! abstract "Usage"
    ```
    Client tool for Scientific Workflow Infrastructure (SciWIn)

    Usage: s4n <COMMAND>

    Commands:
      init      Initializes project folder structure and repository
      tool      Provides commands to create and work with CWL CommandLineTools
      workflow  Provides commands to create and work with CWL Workflows
      annotate  
      execute   Execution of CWL Files locally or on remote servers [aliases: ex]
      sync      
      completions  Generate shell completions
      help      Print this message or the help of the given subcommand(s)

    Options:
      -h, --help     Print help
      -V, --version  Print version
    ```

## Shell completions
Shell completions are available using the `s4n completions` command
!!! abstract "Usage"
    ``` 
    Generate shell completions

    Usage: s4n completions <SHELL>

    Arguments:
      <SHELL>  [possible values: bash, elvish, fish, powershell, zsh]

    Options:
      -h, --help  Print help
    ```
The command can be used to generate the shell completions for several shells.
```
s4n completions bash > completions.sh
```
