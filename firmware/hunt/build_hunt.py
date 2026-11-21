import base64
import hashlib
import struct
import sys
import textwrap
import tomllib
from pathlib import Path

import crackme_secret

HUNT_DIR = Path(__file__).resolve().parent
FIRMWARE_DIR = HUNT_DIR.parent
CONFIG = HUNT_DIR / "hunt.toml"
OUTPUT = FIRMWARE_DIR / "rs" / "hunt.bin"
CRACKME_HEADER = FIRMWARE_DIR / "apps" / "crackme" / "main" / "crackme_secret.h"
CRACKME_ELF = FIRMWARE_DIR / "apps" / "crackme" / "build" / "crackme.app.elf"

MAGIC = b"HUNT"
KEY_LEN = 32
TAG_LEN = 16
CLUE_COLUMNS = 42
CLUE_MAX_LINES = 7
MIN_ANSWER_LEN = 8
WPA_MIN_LEN = 8
WPA_MAX_LEN = 63

FIELD_CLUE_LINE = 1
FIELD_SERIAL = 2
FIELD_MORSE = 3
FIELD_NVS = 4
FIELD_PROGRAM = 5
FIELD_IR_SWAP = 7
FIELD_IR_REMOTE = 8
FIELD_FINISH = 9
FINISH_CLUE_MAX_LINES = 4

PUBLIC_LABEL = b"public"
SWAP_BADGES_MAX = 8
UNTYPEABLE = set("`~")
MORSE_CHARS = set("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789- ")


def normalize(answer):
    return answer.strip().lower()


def derive_key(answer, salt, iterations):
    return hashlib.pbkdf2_hmac("sha256", normalize(answer).encode(), salt.encode(), iterations, KEY_LEN)


def tag(key):
    return hashlib.sha256(key + b"tag").digest()[:TAG_LEN]


def keystream(key, length):
    blocks = (length + KEY_LEN - 1) // KEY_LEN
    return b"".join(hashlib.sha256(key + b"enc" + struct.pack(">I", i)).digest() for i in range(blocks))[:length]


def encrypt(key, plain):
    return bytes(a ^ b for a, b in zip(plain, keystream(key, len(plain))))


def field(kind, data):
    return struct.pack("<BI", kind, len(data)) + data


def clue_fields(text, max_lines=CLUE_MAX_LINES):
    lines = textwrap.wrap(text, CLUE_COLUMNS)
    if len(lines) > max_lines:
        raise ValueError(f"clue too long ({len(lines)} lines, max {max_lines}): {text}")
    return b"".join(field(FIELD_CLUE_LINE, line.encode()) for line in lines)


def stage_fields(stage):
    out = clue_fields(stage["clue"], FINISH_CLUE_MAX_LINES if stage.get("finish") else CLUE_MAX_LINES)
    if swap := stage.get("ir_swap"):
        out += field(FIELD_IR_SWAP, f"{swap['badges']}\n{swap['reveal']}".encode())
    if remote := stage.get("ir_remote"):
        out += field(FIELD_IR_REMOTE, f"{remote['name']}\n{remote['text']}".encode())
    if stage.get("finish"):
        out += field(FIELD_FINISH, b"")
    return out


def badge_fields(config):
    badge = config["badge"]
    return (
        field(FIELD_SERIAL, base64.b64encode(badge["serial"].encode()))
        + field(FIELD_MORSE, badge["morse"].upper().encode())
        + field(FIELD_NVS, badge["nvs"].encode())
        + field(FIELD_PROGRAM, CRACKME_ELF.read_bytes())
    )


def stage_answers(stage):
    return stage.get("answers") or [stage["answer"]]


def all_answers(config):
    return [answer for stage in config["stage"] for answer in stage_answers(stage)]


def badge_codes(config):
    badge = config["badge"]
    swaps = [stage["ir_swap"]["reveal"] for stage in config["stage"] if "ir_swap" in stage]
    return [badge["serial"], badge["morse"], badge["nvs"], badge["crackme"]["reveal"], *swaps]


def check_typeable(answer):
    bad = {c for c in answer if not (" " <= c <= "~") or c in UNTYPEABLE}
    if bad:
        raise ValueError(f"the badge keyboard can not type {sorted(bad)} in {answer!r}")


def check_answers(config):
    answers = [normalize(answer) for answer in all_answers(config)]
    if len(set(answers)) != len(answers):
        raise ValueError("two stages have the same answer")
    for answer in answers:
        check_typeable(answer)
        if len(answer) < MIN_ANSWER_LEN:
            raise ValueError(f"answer too short to resist brute force: {answer!r}")
    for code in badge_codes(config):
        if normalize(code) not in answers:
            raise ValueError(f"the badge gives {code!r} but no stage opens with it")


def check_badge(badge):
    password = badge["crackme"]["reveal"]
    if not (WPA_MIN_LEN <= len(password) <= WPA_MAX_LEN and password == normalize(password)):
        raise ValueError("the crackme reveal is the WPA2 password: 8-63 chars, lowercase, no spaces at the ends")
    if not set(badge["morse"].upper()) <= MORSE_CHARS:
        raise ValueError(f"morse supports A-Z, 0-9, '-' and space only: {badge['morse']!r}")


def check_unlocks(stages):
    swaps = [stage["ir_swap"] for stage in stages if "ir_swap" in stage]
    remotes = [stage["ir_remote"] for stage in stages if "ir_remote" in stage]
    if len(swaps) != 1 or len(remotes) != 1:
        raise ValueError("exactly one stage must unlock ir_swap and one must unlock ir_remote")
    if not 1 <= swaps[0]["badges"] <= SWAP_BADGES_MAX:
        raise ValueError(f"ir_swap badges must be 1-{SWAP_BADGES_MAX}")
    text, name = remotes[0]["text"], remotes[0]["name"]
    if not text.strip() or not text.isascii():
        raise ValueError("ir_remote text must be non-empty ASCII")
    if not name.strip() or "\n" in name:
        raise ValueError("ir_remote name must be one non-empty line")
    swap_stage = next(stage for stage in stages if "ir_swap" in stage)
    if normalize(swaps[0]["reveal"]) in map(normalize, stage_answers(swap_stage)):
        raise ValueError("the ir_swap reveal must open a later stage, not its own")


def check_config(config):
    check_answers(config)
    check_badge(config["badge"])
    check_unlocks(config["stage"])


def pack(config):
    salt, iterations = config["salt"], config["iterations"]
    out = MAGIC + struct.pack("<I", iterations) + struct.pack("<H", len(salt)) + salt.encode()
    public = encrypt(public_key(salt), badge_fields(config))
    out += struct.pack("<IH", len(public), len(all_answers(config))) + public
    for stage in config["stage"]:
        for answer in stage_answers(stage):
            key = derive_key(answer, salt, iterations)
            body = encrypt(key, stage_fields(stage))
            out += tag(key) + struct.pack("<I", len(body)) + body
    return out


def public_key(salt):
    return hashlib.sha256(salt.encode() + PUBLIC_LABEL).digest()


def finish_answer(config):
    return stage_answers(next(stage for stage in config["stage"] if stage.get("finish")))[0]


def finish_code(answer, board_name):
    digest = hashlib.sha256(b"finish" + normalize(answer).encode() + normalize(board_name).encode()).hexdigest().upper()[:8]
    return f"{digest[:4]}-{digest[4:]}"


def load_config():
    config = tomllib.loads(CONFIG.read_text())
    check_config(config)
    return config


def main():
    command = sys.argv[1] if len(sys.argv) > 1 else ""
    config = load_config()
    if command == "crackme":
        crackme_secret.write_header(config["badge"]["crackme"], CRACKME_HEADER)
    elif command == "pack":
        OUTPUT.write_bytes(pack(config))
        print(f"hunt: wrote {OUTPUT} ({OUTPUT.stat().st_size} bytes, {len(all_answers(config))} codes)")
    else:
        sys.exit("usage: build_hunt.py crackme|pack")


if __name__ == "__main__":
    main()
