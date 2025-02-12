import argparse;

parser = argparse.ArgumentParser(description='Echo your input')
parser.add_argument('--message', help='Message to echo', required=True)
parser.add_argument('--output-file', help='File to save the message', required=True)

args = parser.parse_args()

with open(args.output_file, 'w') as f:
    f.write(args.message)
    print(args.message)