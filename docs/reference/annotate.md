# Workflow and Tool Metadata Annotation

The `s4n annotate` command is used to annotate CWL files with metadata (see [CWL documentation: Metadata and Authorship](https://www.commonwl.org/user_guide/topics/metadata-and-authorship.html) and [ARC documentation](https://nfdi4plants.github.io/nfdi4plants.knowledgebase/cwl/cwl-metadata/)). It is recommended to annotate CWL files with minimal information.

!!! abstract "Usage"
    ```
    Annotate CWL files

    Usage: s4n annotate [TOOL_NAME] [COMMAND]

    Commands:
      name         Annotates name of a tool or workflow
      description  Annotates description of a tool or workflow
      license      Annotates license of a tool or workflow
      schema       Annotates schema of a tool or workflow
      namespace    Annotates namespace of a tool or workflow
      author       Annotates author of a tool or workflow (schema.org)
      contributor  Annotates contributor of a tool or workflow (schema.org)
      performer    Annotates performer of a tool or workflow (arc ontology)
      process      Annotates a process (arc ontolology)
      container    Annotates container information of a tool or workflow
      custom       Annotates a CWL file with an custom field and value
      help         Print this message or the help of the given subcommand(s)

      Arguments:
        [TOOL_NAME]  Name of the tool or workflow to annotate
    ```

## `annotate name`

The `s4n annotate name` command annotates a CWL file with a label.
!!! example
    ```
     s4n annotate name main -n "An example tool demonstrating metadata."
    ```
    The command will annotate a CWL file with a label.
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: CommandLineTool

    label: An example tool demonstrating metadata.
    ```  

!!! abstract "Usage"
    ```
    Annotates name of a tool or workflow

    Usage: s4n annotate name --name <NAME> <CWL_NAME>

    Arguments:
      <CWL_NAME>  Name of the CWL file

    Options:
      -n, --name <NAME>  Name of the tool or workflow
      -h, --help         Print help
    ```

## `annotate description`

The `s4n annotate description` command annotates a CWL file with a description.

!!! example
    ```
     s4n annotate description main -d "A description for my example tool"
    ```
    The command will annotate a CWL file with a documentation (doc).
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: CommandLineTool

    doc: A description for my example tool
    ```  

!!! abstract "Usage"
    ```
    Annotates name of a tool or workflow

    Usage: s4n annotate description --description <DESCRIPTION> <CWL_NAME>

    Arguments:
      <CWL_NAME>  Name of the CWL file

    Options:
      -n, --description <DESCRIPTION>  Description of the tool or workflow
      -h, --help         Print help
    ```

## `annotate license`

The `s4n annotate license` command annotates a CWL with a license.

!!! example
    ```
     s4n annotate license main -l "MIT"
    ```
    The command will annotate a CWL file with a license and schema.org namespace and schema.
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: CommandLineTool

    s:license: MIT

    $namespaces:
      s: https://schema.org/

    $schemas:
      - https://schema.org/version/latest/schemaorg-current-https.rdf
    ```  

!!! abstract "Usage"
    ```
    Annotates license of a tool or workflow

    Usage: s4n annotate license --license <LICENSE> <CWL_NAME>

    Arguments:
      <CWL_NAME>  Name of the CWL file

    Options:
      -n, --license <LICENSE>  License of the tool or workflow
      -h, --help         Print help
    ```

## `annotate schema`

The `s4n annotate schema` command annotates a CWL with a schema.

!!! example
    ```
     s4n annotate schema main -s "https://schema.org/version/latest/schemaorg-current-https.rdf"
    ```
    The command will annotate a CWL file with a schema.
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: CommandLineTool

    $schemas:
      - https://schema.org/version/latest/schemaorg-current-https.rdf
    ```  

!!! abstract "Usage"
    ```
    Annotates schema of a tool or workflow

    Usage: s4n annotate schema --schema <SCHEMA> <CWL_NAME>

    Arguments:
      <CWL_NAME>  Name of the CWL file

    Options:
      -n, --schema <SCHEMA>  Schema to annotate
      -h, --help         Print help
    ```
   
## `annotate namespace`

The `s4n annotate namespace` command annotates a CWL with a namespace.

!!! example
    ```
     s4n annotate namespace main -n "https://schema.org/" -s "s"
    ```
    The command will annotate a CWL file with a namespace.
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: CommandLineTool

    $namespaces:
      s: https://schema.org/
    ``` 

!!! abstract "Usage"
    ```
    Annotates schema of a tool or workflow

    Usage: s4n annotate namespace [OPTIONS] --namespace <NAMESPACE> <CWL_NAME>

    Arguments:
      <CWL_NAME>  Name of the CWL file

    Options:
      -n, --namespace <NAMESPACE>  Namespace to annotate
      -s, --short <SHORT>          Namespace abbreviation to annotate
      -h, --help         Print help
    ```

## `annotate author`

The `s4n annotate author` command annotates a CWL with author information (based on schema.org). The minimum requirement is a name for the author, the other fields are optional. If the schema.org namespace and schema are not yet present, they are added to the CWL file.
!!! example
    ```
     s4n annotate author main -n "Jane Doe" -m "doe@mail.de" -i "https://orcid.org/0000-0000-0000-0000"
    ```
    The command will annotate a CWL file with an author.
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: CommandLineTool

    s:author:
      - class: s:Person
        s:identifier: https://orcid.org/0000-0000-0000-0000
        s:email: mailto:doe@mail.de
        s:name: Jane Doe

    $namespaces:
      s: https://schema.org/

    $schemas:
      - https://schema.org/version/latest/schemaorg-current-https.rdf
    ``` 

!!! abstract "Usage"
    ```
    Annotates author of a tool or workflow (schema.org)

    Usage: s4n annotate author [OPTIONS] --name <NAME> <CWL_NAME>

    Arguments:
      <CWL_NAME>  Name of the CWL file

    Options:
      -n, --name <NAME>  Name of the person (author or contributor)
      -m, --mail <MAIL>  Email of the person (author or contributor)
      -i, --id <ID>      Identifier of the person, e.g., ORCID
      -h, --help         Print help
    ```


## `annotate contributor`

The `s4n annotate contributor` command annotates a CWL with contributor information (based on schema.org). The fields are similar to the author fields. If the schema.org namespace and schema are not yet present, they are added to the CWL file.
!!! example
    ```bash
     s4n annotate contributor main -n "John Doe" -m "jdoe@mail.de" -i "http://orcid.org/0000-0000-0000-0001"
    ```
    The command will annotate a CWL file with a contributor.
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: CommandLineTool

    s:contributor:
      - class: s:Person
        s:identifier: https://orcid.org/0000-0000-0000-0001
        s:email: mailto:jdoe@mail.de
        s:name: John Doe

    $namespaces:
    s: https://schema.org/

    $schemas:
      - https://schema.org/version/latest/schemaorg-current-https.rdf
    ``` 

!!! abstract "Usage"
    ```
    Annotates author of a tool or workflow (schema.org)

    Usage: s4n annotate contributor [OPTIONS] --name <NAME> <CWL_NAME>

    Arguments:
      <CWL_NAME>  Name of the CWL file

    Options:
      -n, --name <NAME>  Name of the person (author or contributor)
      -m, --mail <MAIL>  Email of the person (author or contributor)
      -i, --id <ID>      Identifier of the person, e.g., ORCID
      -h, --help         Print help
    ```

## `annotate performer`

The `s4n annotate performer` command annotates a CWL with performer information (based on ARC schema). According the [ARC documentation](https://nfdi4plants.github.io/nfdi4plants.knowledgebase/cwl/cwl-metadata/), a performer can be an individual or teams behind the development or execution of the workflow. The minimum requirement is providing a first and a last name for the performer, the other fields are optional. The role field can be annotated with an ontology. If the ARC namespace and schema are not yet present, they are added to the CWL file.
!!! example
    ```
     s4n annotate performer main -f "John" -l "Doe" -m "jdoe@mail.de" -a "institution1" -r "data scientist"
    ```
    The command will annotate a CWL file with a performer.
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: CommandLineTool

    arc:performer:
    - class: arc:Person
      arc:first name: John
      arc:last name: Doe
      arc:email: jdoe@mail.de
      arc:affiliation: institution1
      arc:has role:
      - class: arc:role
        arc:term accession: http://purl.obolibrary.org/obo/OCCO_15205100
        arc:annotation value: data scientist

      $namespaces:
        arc: https://github.com/nfdi4plants/ARC_ontology

      $schemas:
        - https://raw.githubusercontent.com/nfdi4plants/ARC_ontology/main/ARC_v2.0.owl
    ``` 

!!! abstract "Usage"
    ```
    Annotates performer of a tool or workflow (arc ontology)

    Usage: s4n annotate performer [OPTIONS] --first_name <FIRST_NAME> --last_name <LAST_NAME> <CWL_NAME>

    Arguments:
      <CWL_NAME>  Name of the CWL file

    Options:
      -f, --first_name <FIRST_NAME>    First name of the performer
      -l, --last_name <LAST_NAME>      Last name of the performer
      -m, --mail <MAIL>                Email of the performer
      -a, --affiliation <AFFILIATION>  Affiliation of the performer
      -r, --role <ROLE>                Role of the performer
      -h, --help                       Print help
    ```

## `annotate process`

The `s4n annotate process` command annotates a CWL file with a process sequence (based on [ARC metadata](https://nfdi4plants.github.io/nfdi4plants.knowledgebase/cwl/cwl-metadata/)). The parameter and value field can be annotated with an ontology. The minimum requirement is providing a name for the process sequence, the other fields are optional. If the ARC namespace and schema are not yet present, they are added to the CWL file.
!!! example
    ```
     s4n annotate process main -n "script.py" -i "data/input.txt" -o "results/output.txt" -p "Data transformation" -v "Addition"
    ```
    The command will annotate a CWL file with a process sequence.
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: CommandLineTool

    arc:has process sequence:
    - class: arc:process sequence
      arc:name: script.py
      arc:has input:
      - class: arc:data
        arc:name: data/input.txt
      arc:has output:
      - class: arc:data
        arc:name: results/output.txt
      arc:has parameter value:
      - class: arc:process parameter value
        arc:has parameter:
        - class: arc:protocol parameter
          arc:has parameter name:
          - class: arc:parameter name
            arc:term accession: http://purl.obolibrary.org/obo/NCIT_C43582
            arc:term source REF: ncit
            arc:annotation value: Data Transformation
      arc:value:
        - class: arc:ontology annotation
          arc:term accession: http://purl.obolibrary.org/obo/REX_0000089
          arc:term source REF: rex
          arc:annotation value: addition

    $namespaces:
      arc: https://github.com/nfdi4plants/ARC_ontology

    $schemas:
      - https://raw.githubusercontent.com/nfdi4plants/ARC_ontology/main/ARC_v2.0.owl
    ``` 

!!! abstract "Usage"
    ```
    Annotates a process sequence (arc ontolology)

    Usage: s4n annotate process [OPTIONS] --name <NAME> <CWL_NAME>

    Arguments:
      <CWL_NAME>  Name of the CWL file

    Options:
      -n, --name <NAME>            Name of the process sequence step
      -i, --input <INPUT>          Input file or directory, e.g., folder/input.txt
      -o, --output <OUTPUT>        Output file or directory, e.g., folder/output.txt
      -p, --parameter <PARAMETER>  Process step parameter
      -v, --value <VALUE>          Process step value
      -h, --help                   Print help
    ```

## `annotate container`

The `s4n annotate container` command annotates a CWL file with container information.
!!! example
    ```
     s4n annotate container main -c "Docker container" 
    ```
    The command will annotate a CWL file with the container annotation.
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: CommandLineTool

    arc:has technology type:
    - class: arc:technology type
      arc:annotation value: Docker container

    $namespaces:
      arc: https://github.com/nfdi4plants/ARC_ontology

    $schemas:
      - https://raw.githubusercontent.com/nfdi4plants/ARC_ontology/main/ARC_v2.0.owl
    ``` 

!!! abstract "Usage"
    ```
    Annotates container information of a tool or workflow

    Usage: s4n annotate container --container <CONTAINER> <CWL_NAME>

    Arguments:
      <CWL_NAME> Name of the CWL file

    Options:
      -c, --container <CONTAINER>  Annotation value for the container
      -h, --help                   Print help
    ```

## `annotate custom`

The `s4n annotate custom` command annotates a CWL file with a custom field and value.
!!! example
    ```
     s4n annotate custom main "s:programmingLanguage" "python"
    ```
    The command will annotate a CWL file with a programmingLanguage annotation.
    ```yaml
    #!/usr/bin/env cwl-runner

    cwlVersion: v1.2
    class: CommandLineTool

    s:programmingLanguage: python
    ``` 

!!! abstract "Usage"
    ```
    Annotates a CWL file with an custom field and value

    Usage: s4n annotate custom <CWL_NAME> <FIELD> <VALUE>

    Arguments:
      <CWL_NAME>  Name of the CWL file
      <FIELD>     Field to annotate
      <VALUE>     Value for the field

    Options:
      -h, --help  Print help
    ```

