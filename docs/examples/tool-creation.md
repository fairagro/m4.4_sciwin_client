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

## Implicit inputs - hardcoded files
Like shown in the above example there is also the possibility to specify inputs explictly. This is needed e.g. if the scripts loads a hardcoded file like in the following example.

=== ":octicons-terminal-16: Command"
    ```
    s4n tool create -i file.txt -o out.txt python load.py
    ```
=== ":simple-python: load.py"
    ```python
    with open('file.txt', 'r') as file:
        data = file.read()
        with open('out.txt', 'w') as out:
            out.write(data)
    ```
=== ":simple-commonworkflowlanguage: load.cwl"
    ```yaml
    #!/usr/bin/env cwl-runner
    
    cwlVersion: v1.2
    class: CommandLineTool
    
    requirements:
    - class: InitialWorkDirRequirement
      listing:
      - entryname: file.txt
        entry:
          $include: '../../file.txt'
      - entryname: load.py
        entry:
          $include: '../../load.py'
    
    inputs: []
    outputs:
    - id: out
      type: File
      outputBinding:
        glob: out.txt
    
    baseCommand:
    - python
    - load.py
    ```

## Piping 
Using the pipe operator `|` is a common usecase when using the commandline. Let's assume the first 5 lines of a file are needed e.g `cat speakers.csv | head -n 5 > speakers_5.csv`

=== ":octicons-terminal-16: Command"
    ```
    s4n tool create cat speakers.csv \| head -n 5 \> speakers_5.csv
    ```
=== ":simple-commonworkflowlanguage: cat.cwl"
    ```yaml
       #!/usr/bin/env cwl-runner
       
       cwlVersion: v1.2
       class: CommandLineTool
       
       requirements:
       - class: ShellCommandRequirement
       
       inputs:
       - id: speakers_csv
         type: File
         default:
           class: File
           location: '../../speakers.csv'
         inputBinding:
           position: 0
       
       outputs:
       - id: speakers_5
         type: File
         outputBinding:
           glob: speakers_5.csv
       
       baseCommand: cat
       arguments:
       - position: 1
         valueFrom: '|'
         shellQuote: false
       - position: 1
         valueFrom: head
       - position: 2
         valueFrom: '-n'
       - position: 3
         valueFrom: '5'
       - position: 4
         valueFrom: '>'
       - position: 5
         valueFrom: speakers_5.csv
    ```


## Pulling containers
For full reproducibility it is recommended to use containers e.g. `docker` as requirement inside of the CWL files. Adding an existing container image is quite easy. The `s4n tool create` command needs to be called using `-c` or `--container-image` argument. For testing a python script using `pandas` is used together with the `pandas/pandas` container.

=== ":octicons-terminal-16: Command"
    ```
    s4n tool create -c pandas/pandas:pip-all python calculation.py --population population.csv --speakers speakers_revised.csv
    ```
=== ":simple-python: calculation.py"
    ```python
    import pandas as pd
    import argparse
    
    parser = argparse.ArgumentParser(prog="python calculation.py", description="Calculates the percentage of speakers for each language")
    parser.add_argument("-p", "--population", required=True, help="Path to the population.csv File")
    parser.add_argument("-s", "--speakers", required=True, help="Path to the speakers.csv File")
    
    args = parser.parse_args()
    
    df = pd.read_csv(args.population)
    sum = df["population"].sum()
    
    print(f"Total population: {sum}")
    
    df = pd.read_csv(args.speakers)
    df["percentage"] = df["speakers"] / sum * 100
    
    df.to_csv("results.csv")
    print(df.head(10))
    ```
=== ":simple-commonworkflowlanguage: calculation.cwl"
    ```yaml
    #!/usr/bin/env cwl-runner
    
    cwlVersion: v1.2
    class: CommandLineTool
    
    requirements:
    - class: InitialWorkDirRequirement
      listing:
      - entryname: calculation.py
        entry:
          $include: '../../calculation.py'
    - class: DockerRequirement
      dockerPull: pandas/pandas:pip-all
    
    inputs:
    - id: population
      type: File
      default:
        class: File
        location: '../../population.csv'
      inputBinding:
        prefix: '--population'
    - id: speakers
      type: File
      default:
        class: File
        location: '../../speakers_revised.csv'
      inputBinding:
        prefix: '--speakers'
    
    outputs:
    - id: results
      type: File
      outputBinding:
        glob: results.csv
    
    baseCommand:
    - python
    - calculation.py
    ```
=== "population.csv"
    ```csv
    country,population
    Afghanistan,37466414
    Albania,2793592
    Algeria,43900000
    Andorra,85101
    Angola,32866270
    Antigua and Barbuda,101489
    Argentina,47327407
    Armenia,2930450
    Aruba,106739
    Australia,26473055
    Austria,8979894
    Azerbaijan,10145212
    Bahrain,1311134
    Bangladesh,169356251
    Basque Country,3193513
    Belarus,9155978
    Belgium,11584008
    Benin,11175692
    Bhutan,787424
    Bolivia,11051600
    Botswana,2291661
    Brazil,203062512
    British North Borneo,285000
    Brunei,428697
    Bulgaria,7000039
    Burkina Faso,20488000
    Burundi,11530580
    Cambodia,16005373
    Cameroon,24053727
    Canada,36991981
    Cape Verde,555988
    Catalonia,7747709
    Central African Republic,4659080
    Chad,15477751
    Chile,19458000
    Colombia,49065615
    Comoros,902348
    Cook Islands,17434
    Costa Rica,5044197
    Croatia,3871833
    Cuba,11181595
    Cyprus,1141166
    Czech Republic,10900555
    Democratic Republic of the Congo,86790567
    Denmark,5827463
    Djibouti,956985
    Dominica,74656
    Dominican Republic,10760028
    East Timor,1243235
    Ecuador,16938986
    Egypt,94798827
    El Salvador,5744113
    England,57106398
    Eritrea,3497000
    Estonia,1374687
    Ethiopia,104957438
    Federated States of Micronesia,105544
    Fiji,905502
    Finland,5608218
    France,68373433
    Gabon,2025137
    Galicia,2695645
    Germany,84358845
    Ghana,32833031
    Greece,10482487
    Grenada,114299
    Guatemala,17263239
    Guinea,12717176
    Guinea-Bissau,1861283
    Guyana,777859
    Honduras,10062994
    Hungary,9599744
    Iceland,364260
    India,1326093247
    Indonesia,275439000
    Iran,86758304
    Iraq,38274618
    Israel,9840000
    Italy,58850717
    Ivory Coast,24294750
    Jamaica,2697983
    Japan,125440000
    Jordan,10428241
    Kazakhstan,19002586
    Kenya,48468138
    Kingdom of Denmark,5930987
    Kingdom of the Netherlands,17100715
    Kosovo,1883018
    Kyrgyzstan,6694200
    Laos,6858160
    Latvia,1871882
    Lebanon,6100075
    Lesotho,2007201
    Liberia,5214030
    Libya,6678567
    Liechtenstein,37922
    Lithuania,2860002
    Madagascar,25570895
    Malawi,18622104
    Malaysia,32447385
    Maldives,436330
    Mali,20250833
    Malta,553214
    Mauritania,4614974
    Mauritius,1264613
    Mexico,124777324
    Mongolia,3409939
    Montenegro,622359
    Morocco,37076584
    Mozambique,29668834
    Myanmar,53370609
    Namibia,2533794
    Nauru,13650
    Nepal,29164578
    Netherlands,17590672
    New Zealand,5118700
    Nicaragua,5142098
    Niger,21477348
    Nigeria,211400708
    Niue,1612
    North Korea,25490965
    North Macedonia,1836713
    Northern Ireland,1852168
    Northern Mariana Islands,47329
    Norway,5550203
    Oman,4829480
    Pakistan,223773700
    Palau,21729
    Papua New Guinea,8935000
    Paraguay,6811297
    People's Republic of China,1442965000
    Peru,29381884
    Philippines,109035343
    Poland,38382576
    Portugal,10347892
    Qatar,2639211
    Republic of Ireland,5123536
    Republic of the Congo,5260750
    Romania,19053815
    Russia,145975300
    Rwanda,13246394
    Saint Kitts and Nevis,55345
    Saint Lucia,167591
    Saint Vincent and the Grenadines,109897
    Samoa,200010
    Saudi Arabia,33000000
    Scotland,5404700
    Senegal,16876720
    Seychelles,95843
    Sierra Leone,7557212
    Singapore,5866139
    Sint Maarten,43847
    Slovakia,5449270
    Slovenia,2066880
    Solomon Islands,611343
    Somalia,11031386
    South Africa,62027503
    South Korea,51466201
    South Sudan,12575714
    Spain,47415750
    Sri Lanka,21444000
    State of Palestine,5227193
    Sudan,40533330
    Sweden,10551707
    Switzerland,8902308
    Syria,22933531
    São Tomé and Príncipe,204327
    Taiwan,23412899
    Tajikistan,8921343
    Tanzania,57310019
    Thailand,66188503
    The Bahamas,395361
    The Gambia,2639916
    Togo,7797694
    Trinidad and Tobago,1369125
    Tunisia,11565204
    Turkey,85372377
    Turkmenistan,6117933
    Tuvalu,11792
    Uganda,47123531
    United Arab Emirates,9890400
    United Kingdom,67326569
    United States of America,332278200
    Uruguay,3444263
    Uzbekistan,34915100
    Vanuatu,300019
    Vatican City,764
    Venezuela,28515829
    Vietnam,96208984
    Wales,3113000
    Yemen,28250420
    Zambia,17094130
    Zimbabwe,15178979
    ```
=== "speakers_revised.csv"
    ```csv
    language,speakers
    Bangla,300000000
    Egyptian Arabic,100542400
    English,1132366680
    German,134993040
    Indonesian,198996550
    Japanese,128000000
    Portuguese,475300000
    Punjabi,125000000
    Russian,154000000
    Standard Mandarin,1090951810
    ``` 
=== "results.csv"
    ```csv
    ,language,speakers,percentage
    0,Bangla,300000000,3.8990180176129665
    1,Egyptian Arabic,100542400,1.3067220971134996
    2,English,1132366680,14.71706029288192
    3,German,134993040,1.7544676507078263
    4,Indonesian,198996550,2.5863037796427317
    5,Japanese,128000000,1.663581020848199
    6,Portuguese,475300000,6.177344212571477
    7,Punjabi,125000000,1.6245908406720693
    8,Russian,154000000,2.0014959157079892
    9,Standard Mandarin,1090951810,14.178802545124924
    ``` 

When the tool is executed by a runner supporting containerization e.g. `cwltool` it is using the `pandas/pandas:pip-all` container to run the script in a reproducible environment.

## Building custom containers
Using a complex research environments a custom container is may needed. The same example from above will be executed in a container built from a `Dockerfile`.
This can be achieved by using the `-c` argument with a path to a `Dockerfile`. A tag can be specified by using `-t`.

=== ":octicons-terminal-16: Command"
    ```
    s4n tool create -c Dockerfile -t my-docker python calculation.py --population population.csv --speakers speakers_revised.csv
    ```
=== ":simple-commonworkflowlanguage: calculation.cwl"
    ```yaml
    #!/usr/bin/env cwl-runner
    
    cwlVersion: v1.2
    class: CommandLineTool
    
    requirements:
    - class: InitialWorkDirRequirement
      listing:
      - entryname: calculation.py
        entry:
          $include: '../../calculation.py'
    - class: DockerRequirement
      dockerFile:
        $include: '../../Dockerfile'
      dockerImageId: my-docker
    
    inputs:
    - id: population
      type: File
      default:
        class: File
        location: '../../population.csv'
      inputBinding:
        prefix: '--population'
    - id: speakers
      type: File
      default:
        class: File
        location: '../../speakers_revised.csv'
      inputBinding:
        prefix: '--speakers'
    
    outputs:
    - id: results
      type: File
      outputBinding:
        glob: results.csv
    
    baseCommand:
    - python
    - calculation.py
    
    ```
=== ":simple-docker: Dockerfile"
    ```docker
    FROM python
    RUN pip install pandas
    ```

A runner will check whether a container with the specified image is existent and build it using the `Dockerfile` otherwise.
