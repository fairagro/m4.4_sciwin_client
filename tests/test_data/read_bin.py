import struct

with open("data.bin", "rb") as f:
    data = f.read()
    number = int(struct.unpack("d", data)[0])

with open("output.txt", "w") as f:
    f.write(str(number))