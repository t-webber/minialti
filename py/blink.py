from machine import Pin
from utime import sleep

pin = Pin(25, Pin.OUT)

def on(a, b):
    pin.value(1)
    sleep(a)
    pin.value(0)
    sleep(b)

while True:
    on(0.2, 0.2)
    on(0.8, 0.8)
