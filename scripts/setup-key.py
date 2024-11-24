#!/usr/bin/env python

import os
import keyboard
import struct
import time

from hid_code import codes_table

if os.name == 'nt':
    import ctypes
    # preload dll to ensure success link in windows
    ctypes.CDLL(f'{os.path.dirname(os.path.abspath(__file__))}/hidapi.dll')

import hid

DEFAULT_VID = 0x16c0
DEFAULT_PID = 0x314f

DEFAULT_PATH=b'\\\\?\\HID#VID_16C0&PID_314F&MI_01#8&1f6c5a59&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}'


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
    # for d in hid.enumerate(DEFAULT_VID, DEFAULT_PID):
    #     print(d)
        
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
    
    print(f"Raw data: {data}")
        
    with hid.Device(path=DEFAULT_PATH) as h:
        h.write(data)

        time.sleep(0.5)

        res = h.read(len(data))
        if res == data[1:]:
            print("Applyed succesfuly!")
        else:
            print(f"Failed to apply hotkey ({res})")


if __name__ == '__main__':
    main()