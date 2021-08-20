#!/usr/bin/env python3

from sys import argv

# Can't be bothered to figure out how to make Python auto-derive base from 0x/0b etc,
# so I do this instead
num = eval(argv[1])

print("Bits set:")
for i in range(128):
    if num & (1 << i):
        print("  ", i)
