import os

with open(os.path.join(os.path.dirname(__file__), "input.txt"), "r") as i:
    with open("results.txt", "w") as o:
        o.write(i.read())
