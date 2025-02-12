from time import sleep

sleep(60)

with open('sleep.txt', 'w') as f:
    f.write('I slept for 60 seconds')