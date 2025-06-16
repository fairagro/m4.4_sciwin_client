import matplotlib
from matplotlib import pyplot as plt
import pandas as pd
import argparse
import scienceplots
assert scienceplots, 'scienceplots needed'
plt.style.use(['science', 'no-latex'])

parser = argparse.ArgumentParser(prog="python plot.py", description="Plots the percentage of speakers for each language")
parser.add_argument("-r", "--results", required=True, help="Path to the results.csv File")
args = parser.parse_args()

df = pd.read_csv(args.results)
colors = matplotlib.colormaps['tab10'](range(len(df)))

ax = df.plot.bar(x='language', y='percentage', legend=False, title='Language Popularity', color=colors) 
ax.yaxis.set_label_text('Percentage (%)')
ax.xaxis.set_label_text('')

plt.savefig('results.svg')