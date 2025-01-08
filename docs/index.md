

## Example Usage
Add `s4n` to your `PATH` environment variable if not done already. 
```bash
export PATH=$PATH:/path/to/your/s4n/executable
```

To verify the successful addition to the `PATH` variable the following command can be used.
```bash
s4n -V
# s4n 0.1.0
```

To initialize a new project use the `s4n init` command. A project folder can be specifies using the `-p` argument.
The command will initialize a git repository in this folder if there is none already. Furthermore a `workflows` folder will be created.
```bash 
s4n init -p test_project
# 📂 s4n project initialisation sucessfully:
# test_project (Base)
#   ├── workflows
```

For this example some data needs to be created. To download the data a new folder `data` needs to be created. The raw data files can be downloaded using e.g. `wget`.
```bash
wget https://raw.githubusercontent.com/fairagro/m4.4_sciwin_client/refs/heads/main/tests/test_data/hello_world/data/population.csv
wget https://raw.githubusercontent.com/fairagro/m4.4_sciwin_client/refs/heads/main/tests/test_data/hello_world/data/speakers_revised.csv
```

The keep the demo project organized, the `workflows` folder will also be used to house the scripts used in this demo. The following bash script needs to be created as `workflows/calculation/calculation.sh` 
```bash
#!/bin/bash

# Function to display usage
usage() {
    echo "Usage: $0 --population <population_csv> --speakers <speakers_csv>"
    exit 1
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --population)
            population_file="$2"
            shift 2
            ;;
        --speakers)
            speakers_file="$2"
            shift 2
            ;;
        *)
            usage
            ;;
    esac
done

# Validate arguments
if [[ -z "$population_file" || -z "$speakers_file" ]]; then
    usage
fi

# Check if the files exist
if [[ ! -f "$population_file" || ! -f "$speakers_file" ]]; then
    echo "Error: One or both files not found!"
    exit 1
fi

# Calculate total population
total_population=$(awk -F',' 'NR > 1 {sum += $2} END {print sum}' "$population_file")

# Validate total_population
if [[ -z "$total_population" || "$total_population" -eq 0 ]]; then
    echo "Error: Invalid total population value!"
    exit 1
fi

# Calculate percentage for each language
echo "Language,Speakers,Percentage" # Header for output
awk -v total="$total_population" -F',' 'NR > 1 {percentage = ($2 / total) * 100; printf "%s,%d,%.2f%%\n", $1, $2, percentage}' "$speakers_file"
```

To run the tool creation command the changes need to be committed beforehand. The shell script usually would be called with the command `workflows/calculation/calculate_percentage.sh --speakers data/speakers_revised.csv --population data/population.csv > results.csv`. To create a CommandLineTool this only needs to be prefixed with `s4n tool create` or `s4n run`. However the `>` operator needs to be escaped using a backslash.
```bash 
s4n tool create workflows/calculation/calculate_percentage.sh --speakers data/speakers_revised.csv --population data/population.csv \> results.csv
```
