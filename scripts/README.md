## Установка комбинации клавиш для one-button-keyboard

Требуется [python3](https://www.python.org/downloads/) и установленные библитеки
- [`hid`](https://pypi.org/project/hid/)
- [`keyboard`](https://pypi.org/project/keyboard/)

Запускать предпочтительно в [venv](https://python.land/virtual-environments/virtualenv)

## Установка

0. Создать `venv` и зайти в него

    ```ps
    python -m venv venv
    ./venv/scripts/Activate.ps1
    ```
    
1. Установить зависимости

    ```powershell
    pip install -r requirements.txt
    ```

3. (Windows) Установить библиотеку [hidapi.dll](https://github.com/libusb/hidapi/releases)
    Должна лежать рядом с выполняемым скриптом [setup-key.py](setup-key.py)
