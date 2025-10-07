with open('file.txt', 'r') as file:
    data = file.read()
    with open('out.txt', 'w') as out:
        out.write(data)