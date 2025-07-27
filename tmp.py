import sys
import termios
import tty

def main():
    print("Press keys (Ctrl-C to exit):")
    fd = sys.stdin.fileno()
    old_settings = termios.tcgetattr(fd)

    try:
        tty.setraw(fd)  # 入力を生で取得（エンター不要）
        while True:
            ch = sys.stdin.read(1)  # 1バイト読み取り
            print(f"Key: {repr(ch)} | Ordinal: {ord(ch)} | Hex: {hex(ord(ch))}")
    except KeyboardInterrupt:
        print("\nExiting.")
    finally:
        termios.tcsetattr(fd, termios.TCSADRAIN, old_settings)

if __name__ == "__main__":
    main()