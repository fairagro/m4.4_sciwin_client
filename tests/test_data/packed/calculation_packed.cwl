{
    "class": "CommandLineTool",
    "requirements": [
        {
            "class": "InitialWorkDirRequirement",
            "listing": [
                {
                    "entryname": "workflows/calculation/calculation.py",
                    "entry": "import pandas as pd\nimport argparse\n\nparser = argparse.ArgumentParser(prog=\"python calculation.py\", description=\"Calculates the percentage of speakers for each language\")\nparser.add_argument(\"-p\", \"--population\", required=True, help=\"Path to the population.csv File\")\nparser.add_argument(\"-s\", \"--speakers\", required=True, help=\"Path to the speakers.csv File\")\n\nargs = parser.parse_args()\n\ndf = pd.read_csv(args.population)\nsum = df[\"population\"].sum()\n\nprint(f\"Total population: {sum}\")\n\ndf = pd.read_csv(args.speakers)\ndf[\"percentage\"] = df[\"speakers\"] / sum * 100\n\ndf.to_csv(\"results.csv\")\nprint(df.head(10))"
                }
            ]
        },
        {
            "class": "DockerRequirement",
            "dockerPull": "pandas/pandas:pip-all"
        }
    ],
    "inputs": [
        {
            "id": "#main/population",
            "type": "File",
            "default": {
                "class": "File",
                "location": "file:///mnt/m4.4_sciwin_client/tests/test_data/hello_world/data/population.csv"
            },
            "inputBinding": {
                "prefix": "--population"
            }
        },
        {
            "id": "#main/speakers",
            "type": "File",
            "default": {
                "class": "File",
                "location": "file:///mnt/m4.4_sciwin_client/tests/test_data/hello_world/data/speakers_revised.csv"
            },
            "inputBinding": {
                "prefix": "--speakers"
            }
        }
    ],
    "outputs": [
        {
            "id": "#main/results",
            "type": "File",
            "outputBinding": {
                "glob": "results.csv"
            }
        }
    ],
    "baseCommand": [
        "python",
        "workflows/calculation/calculation.py"
    ],
    "id": "#main",
    "cwlVersion": "v1.2"
}