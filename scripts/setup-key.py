#!/usr/bin/env python

import os

if os.name == 'nt':
    import ctypes
    ctypes.CDLL(f'{os.path.dirname(os.path.abspath(__file__))}/hidapi.dll')

import hid


def main():
    pass


if __name__ == '__main__':
    main()