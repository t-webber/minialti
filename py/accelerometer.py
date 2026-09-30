import struct

import machine
import utime

ACCEL_ADDR = 0x1E  # SA0 low

WHO_AM_I = 0x0F
CTRL_REG1 = 0x20
CTRL_REG1_VAL = 0b00011111  # 10Hz, Normal Mode, XYZ enabled
CTRL_REG4 = 0x23
CTRL_REG4_VAL = 0b00110100  # 8G, ya pas 16G sur cet accéléromètre
OUT_X_L = 0x28

ACCELEROMETER_SCALING_FACTOR = 4096  # Valeur pour 8G

i2c = machine.I2C(0, scl=machine.Pin(1), sda=machine.Pin(0))

who = i2c.readfrom_mem(ACCEL_ADDR, WHO_AM_I, 1)[0]
assert who == 0x41

i2c.writeto_mem(ACCEL_ADDR, CTRL_REG4, bytes([CTRL_REG4_VAL]))
i2c.writeto_mem(ACCEL_ADDR, CTRL_REG1, bytes([CTRL_REG1_VAL]))

assert CTRL_REG4_VAL == i2c.readfrom_mem(ACCEL_ADDR, CTRL_REG4, 1)[0]
assert CTRL_REG1_VAL == i2c.readfrom_mem(ACCEL_ADDR, CTRL_REG1, 1)[0]

print("=== Lectures non scalé ===")
# tolérance x,y 4096 sin(5°) = 357+123
# 123: +-30mg / (0.244mg/digit)
# tolérance z 4096 cos(5°) = 4080
print("Expected à plat: x,y ~ [-480,+480], z ~ [3957,4219]")
raw = i2c.readfrom_mem(ACCEL_ADDR, OUT_X_L, 6)
x, y, z = struct.unpack("<hhh", raw)
print(f"{x=}, {y=}, {z=}")


def get_accel() -> list[float]:
    raw_accel_recv = i2c.readfrom_mem(ACCEL_ADDR, OUT_X_L, 6)
    raw_accel = struct.unpack("<hhh", raw_accel_recv)
    return [val / ACCELEROMETER_SCALING_FACTOR for val in raw_accel]


def col(x: float) -> str:
    x = abs(x)
    return "\x1b[31m" if x > 1 else "\x1b[32m" if x > 0.5 else "\x1b[0m"


x, y, z = get_accel()
while True:
    x2, y2, z2 = get_accel()
    xc, yc, zc = col(x - x2), col(y - y2), col(z - z2)
    print(f"{xc}{x2:+.2f} {yc}{y2:+.2f} {zc}{z2:+.2f}", end="\r")
    utime.sleep(0.1)
    x, y, z = x2, y2, z2
