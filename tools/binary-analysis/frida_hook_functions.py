from __future__ import print_function

import sys

import frida


def on_message(message: str, data):
    print("[%s] => %s" % (message, data))


def main(target_process: str):
    session = frida.attach(target_process)
    with open("_agent.js", "r") as script_file:
        script_text = script_file.read()

    script = session.create_script(script_text)
    script.on('message', on_message)
    script.load()

    print("[!] Ctrl+D on UNIX, Ctrl+Z on Windows/cmd.exe to detach from instrumented program.\n\n")
    sys.stdin.read()
    session.detach()


if __name__ == '__main__':
    if len(sys.argv) != 3:
        print("Usage: %s <process name or PID>" % __file__)
        sys.exit(1)

    try:
        target_process = int(sys.argv[1])
    except ValueError:
        target_process = sys.argv[1]

    main(target_process)
