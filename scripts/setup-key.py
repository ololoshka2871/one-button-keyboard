#!/usr/bin/env python

import os, sys
import keyboard
import struct
import time
import argparse

from hid_code import codes_table

if os.name == 'nt':
    import ctypes
    # preload dll to ensure success link in windows
    ctypes.CDLL(f'{os.path.dirname(os.path.abspath(__file__))}/hidapi.dll')

import hid

DEFAULT_VID = 0x16c0
DEFAULT_PID = 0x314f

def read_hotkey(suppress=True):
    import queue as _queue
    
    queue = _queue.Queue()
    fn = lambda e: queue.put(e) or e.event_type == keyboard.KEY_DOWN
    hooked = keyboard.hook(fn, suppress=suppress)
    while True:
        event = queue.get()
        if event.event_type == keyboard.KEY_UP:
            keyboard.unhook(hooked)
            with keyboard._pressed_events_lock:
                names = [e.name for e in keyboard._pressed_events.values()] + [event.name]
            return names
    
    
def scancode(character) -> int:
    code = codes_table()[character]
    return code
    #return keyboard.key_to_scan_codes(character)[0]
    
        
def main():
    parser = argparse.ArgumentParser(description='Set key for one-button-keyboard')
    parser.add_argument('-l', '--list', action='store_true', help='List hid endpoints for device VID/PID')
    parser.add_argument('--vid', type=int, default=DEFAULT_VID, help=f'Device vendor ID. Default: {DEFAULT_VID}')
    parser.add_argument('--pid', type=int, default=DEFAULT_PID, help=f'Device product ID. Default: {DEFAULT_PID}')
    parser.add_argument('-p', '--path', type=str, help=f'Device path (use `{sys.argv[0]} --list --vid VID --pid PID` to find it)')
    parser.add_argument('-r', '--read', action='store_true', help="Read current combination, needs `--path`")
    
    args = parser.parse_args()
        
    if args.list:  
        print(f"List interfaces of {args.vid}:{args.pid}")
        for d in hid.enumerate(args.vid, args.pid):
            print(f"{d['path'].decode()}")
        exit(0)
        
    if args.path:  
        with hid.Device(path=args.path.encode()) as h:
            if args.read:
                res = h.read(64)
                res = struct.unpack('<BHHH', res)
                print(f"Current setting {res[0]}:{list(res[1:])}")
                exit(0)
            
            print('Press and release your desired shortcut: ')
            shortcut = read_hotkey()
            pressed = set(shortcut)
            print(f'Shortcut selected: {pressed}')
            
            all_modifiers = {
                'ctrl': 1 << 0,
                'shift': 1 << 1,
                'alt': 1 << 2,
                'windows': 1 << 3,
                'left ctrl': 1 << 0,
                'left shift': 1 << 1,
                'left alt': 1 << 2,
                'left windows': 1 << 3,
                'right ctrl': 1 << 4,
                'right shift': 1 << 5,
                'right alt': 1 << 6,
                'right windows': 1 << 7,
            }
            
            modifiers = 0
            for m in all_modifiers.keys():
                if m in pressed:
                    modifiers = modifiers | all_modifiers[m]
                    pressed.remove(m)
                    
            keys = [scancode(key) for key in pressed]
            for i in range(3 - len(keys)):
                keys.append(0)
            
            print(f"Modifiers flags [{modifiers}]:{keys}")
                
            data = struct.pack('<BBHHH', 0, modifiers, *keys)
            
            # write
            h.write(data)
                
if __name__ == '__main__':
    main()