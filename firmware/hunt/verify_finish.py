import sys
import tomllib

from build_hunt import CONFIG, finish_answer, finish_code


def main():
    if len(sys.argv) != 3:
        sys.exit("usage: verify_finish.py <board name> <code>")
    board_name, code = sys.argv[1], sys.argv[2].strip().upper().replace("-", "")
    expected = finish_code(finish_answer(tomllib.loads(CONFIG.read_text())), board_name).replace("-", "")
    print(f"{board_name}: {'VALID' if code == expected else 'WRONG'}")
    sys.exit(0 if code == expected else 1)


if __name__ == "__main__":
    main()
