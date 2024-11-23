#!/usr/bin/env python
# Run this code from root!

import os

if os.name == 'nt':
    import ctypes
    ctypes.CDLL(f'{os.path.dirname(os.path.abspath(__file__))}/hidapi.dll')

import hid

DEFAULT_VID = 0x16c0
DEFAULT_PID = 0x314f

DEFAULT_PATH=b'\\\\?\\HID#VID_16C0&PID_314F&MI_01#8&1f6c5a59&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}'


def main():
    for d in hid.enumerate(DEFAULT_VID, DEFAULT_PID):
        print(d)
    
    with hid.Device(path=DEFAULT_PATH) as h:
        # print(f'Device manufacturer: {h.manufacturer}')
        # print(f'Product: {h.product}')
        # print(f'Serial Number: {h.serial}')
        
        h.write(bytes([1, 1, 2, 20, 4, 5, 60]))


if __name__ == '__main__':
    main()