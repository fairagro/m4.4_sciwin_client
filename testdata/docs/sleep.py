from time import sleep

sleep(30)

with open('sleep.txt', 'w') as f:
    f.write('I slept for 30 seconds')