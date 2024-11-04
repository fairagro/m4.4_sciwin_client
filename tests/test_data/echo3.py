import argparse

parser = argparse.ArgumentParser()
parser.add_argument("-o", "--out", required=True, help="test argument pointing to txt file")
args = parser.parse_args()

with open(args.out, "w") as file:
    file.write("Hello, World!")

print(f"File '{args.out}' has been created with 'Hello, World!' inside.")