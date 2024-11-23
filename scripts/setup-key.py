#!/usr/bin/env python
# Run this code from root!

import os

if os.name == 'nt':
    import ctypes
    ctypes.CDLL(f'{os.path.dirname(os.path.abspath(__file__))}/hidapi.dll')

import hid

DEFAULT_VID = 0x16c0
DEFAULT_PID = 0x314f


def main():
    with hid.Device(DEFAULT_VID, DEFAULT_PID) as h:
        print(f'Device manufacturer: {h.manufacturer}')
        print(f'Product: {h.product}')
        print(f'Serial Number: {h.serial}')
        
        h.send_feature_report(b'0123344')


if __name__ == '__main__':
    main()