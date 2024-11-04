import argparse;

parser = argparse.ArgumentParser()
parser.add_argument("-t", "--test", required=True, help="test argument pointing to txt file")
parser.add_argument("-t2", "--test2", required=True, help="test2 argument pointing to txt file")
parser.add_argument("-o", "--out", required=True, help="test argument pointing to txt file")


args = parser.parse_args()

with open(args.test, "r") as i:
    with open("results.txt", "w") as o:
        o.write(i.read())

with open(args.test, "r") as i:
    with open(args.out, "w") as file:
        file.write(i.read())

