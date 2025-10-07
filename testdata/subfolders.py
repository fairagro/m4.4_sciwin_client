import os

path = os.path.join("media", "data")
os.makedirs(path, exist_ok=True)

with open(path + "/alerta.161", "w") as f:
    f.write("alerta!")