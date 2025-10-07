import argparse;

parser = argparse.ArgumentParser()
parser.add_argument("-t", "--test", required=True, help="test argument pointing to txt file")

args = parser.parse_args()

with open(args.test, "r") as i:
    with open("results.txt", "w") as o:
        o.write(i.read())