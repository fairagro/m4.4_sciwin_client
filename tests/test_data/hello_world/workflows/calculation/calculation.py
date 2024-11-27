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