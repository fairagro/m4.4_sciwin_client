# Workflow and Tool Execution
The `execute` command provides tools to execute CWL documents locally or on a remote server (soonish™).
!!! abstract "Usage"
    ```
    Execution of CWL Files locally or on remote servers

    Usage: s4n execute <COMMAND>

    Commands:
      local  Runs CWL files locally using a custom runner or cwltool [aliases: l]      
      make-template  Creates job file template for execution (e.g. inputs.yaml)
      help   Print this message or the help of the given subcommand(s)

    Options:
      -h, --help  Print help
    ```

## `execute local`
There are two options for local execution. Using the CWL reference runner `cwltool` which needs to be installed as extern dependency using `pip install cwltool` or the SciWIn client runner which supports a large subset of CWL but lacks support containerization. You may ask yourself: why is there a custom runner? Because `cwltool` only supports Windows using the Windows Subsystem for Linux (wsl) which is deactivated on many enterprise systems. The intention is to have a simple tool to test the generated CWL documents before sending them to the remote server.
The usage of the internal runner, which is the default one, is similar to the usage of `cwltool`. It accepts the cwl file as first parameter and the inputs following at the end of the commands either as command line string or yaml file.

!!! abstract "Usage"
    ```
    Runs CWL files locally using a custom runner or cwltool

    Usage: s4n execute local [OPTIONS] <FILE> [ARGS]...

    Arguments:
      <FILE>     CWL File to execute
      [ARGS]...  Other arguments provided to cwl file

    Options:
      -r, --runner <RUNNER>   Choose your cwl runner implementation [default: custom] [possible values: cwltool, custom]
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