# Workflow and Tool Execution
The `execute` command provides tools to execute CWL documents locally or on a remote server (soonish™).
!!! abstract "Usage"
    ```
    Execution of CWL Files locally or on remote servers

    Usage: s4n execute <COMMAND>

    Commands:
      local  Runs CWL files locally [aliases: l]      
      make-template  Creates job file template for execution (e.g. inputs.yaml)
      help   Print this message or the help of the given subcommand(s)

    Options:
      -h, --help  Print help
    ```

## `execute local`
!!! abstract "Usage"
    ```
    Runs CWL files locally

    Usage: s4n execute local [OPTIONS] <FILE> [ARGS]...

    Arguments:
      <FILE>     CWL File to execute
      [ARGS]...  Other arguments provided to cwl file

    Options:
          --outdir <OUT_DIR>  A path to output resulting files to
          --quiet             Runner does not print to stdout
          --podman            Use podman instead of docker
      -h, --help              Print help
    ```


## `execute remote`
Not yet implemented

## `execute make-template`
`s4n execute make-template` is able to create a dummy CWL job file (e.g. inputs.yaml) that can be used as a template for an upoming execution of CWL.
!!! abstract "Usage"
    ```
    Creates job file template for execution (e.g. inputs.yaml)
    
    Usage: s4n execute make-template <CWL>
    
    Arguments:
      <CWL>  CWL File to create input template for
    
    Options:
      -h, --help  Print help
    ```