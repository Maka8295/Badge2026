import argparse
import collections
import pathlib
import random
import re
import tempfile

MOVES = "UDLRA"
UP, DOWN, LEFT, RIGHT, PRESS = range(len(MOVES))
DIRECTIONS = ((UP, 0, -1), (DOWN, 0, 1), (LEFT, -1, 0), (RIGHT, 1, 0))
OPPOSITE = {UP: DOWN, DOWN: UP, LEFT: RIGHT, RIGHT: LEFT}
ALL_WALLS = 0x0F
MARK_SHIFT = 4
MAX_CHECKPOINTS = 15
MIN_MAZE_SIZE = 4
MAX_MAZE_SIZE = 10

MIN_PATH_LENGTH = 20
MAX_PATH_LENGTH = 40
MIN_REVEAL_LENGTH = 8
MAX_REVEAL_LENGTH = 32
FIRST_PRINTABLE = 0x20
LAST_PRINTABLE = 0x7E

WORD_BITS = 32
WORD_MASK = (1 << WORD_BITS) - 1
LONG_MASK = (1 << 64) - 1
BYTE_MASK = 0xFF
CELL_MASK_MULT = 0x9E3779B1
CELL_MASK_ADD = 0x50F6CCCB
CELL_MASK_SHIFT = 24

MOVE_SHIFT = 16
PROGRESS_SHIFT = 8
START_STATE = 0x73880FEE7E4D6413
GOLDEN = 0x9E3779B97F4A7C15
MIX_A = 0xBF58476D1CE4E5B9
MIX_B = 0x94D049BB133111EB
MIX_SHIFT_A = 30
MIX_SHIFT_B = 27
MIX_SHIFT_C = 31
SLOW_ROUNDS = 1 << 21
SEAL_ROUNDS = 64
TAG_SALT = 0x29C3EC25ECCEAE36
KEY_SALT = 0xD4988BA72D84E3F3
BLOCK_BYTES = 8

OPCODE_NAMES = ("set", "add", "mul", "xor", "and", "shr", "load", "jz")
REGISTER_NAMES = ("cell", "progress", "move", "word", "a", "b", "zero")
SHARED_REGISTERS = ("cell", "progress", "move", "word")
REGISTER_COUNT = 8
REGISTER_MASK = REGISTER_COUNT - 1
REGISTER_SHIFT = 4
HEAD_BYTES = 2
IMMEDIATE_BYTES = 4
TARGET_BYTES = 1
EXTRA_BYTES = {"set": IMMEDIATE_BYTES, "jz": TARGET_BYTES}
SHIFT_MASK = 31
BYTE_BITS = 8
BYTE_VALUES = 256
MAX_PROGRAM_LENGTH = BYTE_VALUES
SHUFFLE_SALT = 0x564D2D4F50434F44
LABEL_END = ":"
ODD = 1

PROGRAM_SOURCE = """
    set a {cell_mask_mult}
    mul a cell
    set b {cell_mask_add}
    add a b
    set b {cell_mask_shift}
    shr a b
    load b cell
    xor a b
    set zero 0
    set b {press}
    xor b move
    jz b press
    shr a move
    set b 1
    and a b
    jz a open
    jz zero word
open:
    set a {byte_bits}
    mul a move
    set b {steps}
    shr b a
    set a {byte_mask}
    and b a
    add cell b
    set b {step_bias}
    add cell b
    jz zero word
press:
    set b {mark_shift}
    shr a b
    set b 1
    add b progress
    xor a b
    jz a bump
    jz zero word
bump:
    set a 1
    add progress a
word:
    set word {move_scale}
    mul word move
    set a {progress_scale}
    mul a progress
    add word a
    add word cell
"""

SAMPLE = {
    "reveal": "sample-pass-1234",
    "maze_seed": 6,
    "maze_size": 8,
    "extra_openings": 4,
    "start": [0, 0],
    "checkpoints": [[7, 0], [2, 5], [7, 7]],
}
OPEN_GRID = {**SAMPLE, "extra_openings": 60}
WRONG_PATHS = ("", "A", "RRRRRRRA")


def neighbor(size: int, cell: int, move: int) -> int:
    _, dx, dy = DIRECTIONS[move]
    return cell + dx + dy * size


class Layout:
    def __init__(self, size: int, cells: list[int], start: int, checkpoints: int):
        self.size, self.cells, self.start, self.checkpoints = size, cells, start, checkpoints


class Code:
    def __init__(self, opcodes: dict, registers: dict, mask: tuple[int, int], program: bytes):
        self.opcodes, self.registers, self.mask, self.program = opcodes, registers, mask, program


class Machine:
    def __init__(self, code: Code, table: list[int]):
        self.code, self.table = code, table
        self.names = {number: name for name, number in code.opcodes.items()}

    def fetch(self, pc: int) -> int:
        return self.code.program[pc] ^ code_mask(self.code.mask, pc)

    def immediate(self, pc: int) -> int:
        return int.from_bytes(bytes(self.fetch(pc + i) for i in range(IMMEDIATE_BYTES)), "little")

    def combine(self, name: str, a: int, b: int) -> int:
        results = {"add": a + b, "mul": a * b, "xor": a ^ b, "and": a & b,
                   "shr": a >> (b & SHIFT_MASK), "load": self.table[b % len(self.table)]}
        return results.get(name, a) & WORD_MASK

    def execute(self, reg: list[int], pc: int) -> int:
        name, regs = self.names.get(self.fetch(pc)), self.fetch(pc + 1)
        dst, src = regs & REGISTER_MASK, regs >> REGISTER_SHIFT & REGISTER_MASK
        pc += HEAD_BYTES
        if name == "set":
            reg[dst] = self.immediate(pc)
            return pc + IMMEDIATE_BYTES
        if name == "jz":
            return pc + TARGET_BYTES if reg[dst] else self.fetch(pc)
        reg[dst] = self.combine(name, reg[dst], reg[src])
        return pc

    def run(self, cell: int, progress: int, move: int) -> tuple[int, int, int]:
        at, reg, pc = self.code.registers, [0] * REGISTER_COUNT, 0
        reg[at["cell"]], reg[at["progress"]], reg[at["move"]] = cell, progress, move
        while pc < len(self.code.program):
            pc = self.execute(reg, pc)
        return reg[at["cell"]], reg[at["progress"]], reg[at["word"]]


class Maze:
    def __init__(self, machine: Machine, size: int, start: int, checkpoints: int):
        self.machine, self.size, self.start, self.checkpoints = machine, size, start, checkpoints

    def step(self, cell: int, progress: int, move: int) -> tuple[int, int]:
        return self.machine.run(cell, progress, move)[:2]


def cell_index(size: int, point: list[int]) -> int:
    return point[1] * size + point[0]


def open_wall(cells: list[int], size: int, cell: int, move: int) -> None:
    cells[cell] &= ~(1 << move)
    cells[neighbor(size, cell, move)] &= ~(1 << OPPOSITE[move])


def inside(size: int, cell: int, move: int) -> bool:
    x, y = cell % size, cell // size
    _, dx, dy = DIRECTIONS[move]
    return 0 <= x + dx < size and 0 <= y + dy < size


def unseen_moves(size: int, cell: int, seen: set) -> list[int]:
    moves = [m for m, _, _ in DIRECTIONS if inside(size, cell, m)]
    return [m for m in moves if neighbor(size, cell, m) not in seen]


def carve_tree(rng: random.Random, size: int) -> list[int]:
    cells = [ALL_WALLS] * (size * size)
    stack, seen = [0], {0}
    while stack:
        cell = stack[-1]
        options = unseen_moves(size, cell, seen)
        if not options:
            stack.pop()
            continue
        move = rng.choice(options)
        open_wall(cells, size, cell, move)
        stack.append(neighbor(size, cell, move))
        seen.add(stack[-1])
    return cells


def add_openings(rng: random.Random, cells: list[int], size: int, count: int) -> None:
    walls = [(c, m) for c in range(len(cells)) for m in (DOWN, RIGHT)
             if inside(size, c, m) and cells[c] & (1 << m)]
    for cell, move in rng.sample(walls, min(count, len(walls))):
        open_wall(cells, size, cell, move)


def build_layout(crackme: dict) -> Layout:
    size = crackme["maze_size"]
    rng = random.Random(crackme["maze_seed"])
    cells = carve_tree(rng, size)
    add_openings(rng, cells, size, crackme["extra_openings"])
    for order, point in enumerate(crackme["checkpoints"], 1):
        cells[cell_index(size, point)] |= order << MARK_SHIFT
    return Layout(size, cells, cell_index(size, crackme["start"]), len(crackme["checkpoints"]))


def program_values(size: int) -> dict:
    steps = sum((dx + dy * size + size) << (BYTE_BITS * move) for move, dx, dy in DIRECTIONS)
    return {"cell_mask_mult": CELL_MASK_MULT, "cell_mask_add": CELL_MASK_ADD,
            "cell_mask_shift": CELL_MASK_SHIFT, "press": PRESS, "byte_bits": BYTE_BITS,
            "steps": steps, "byte_mask": BYTE_MASK, "step_bias": -size & WORD_MASK,
            "mark_shift": MARK_SHIFT, "move_scale": 1 << MOVE_SHIFT, "progress_scale": 1 << PROGRESS_SHIFT}


def source_lines(source: str) -> list[list[str]]:
    return [line.split() for line in source.splitlines() if line.strip()]


def instruction_size(name: str) -> int:
    return HEAD_BYTES + EXTRA_BYTES.get(name, 0)


def label_addresses(lines: list[list[str]]) -> dict:
    labels, pc = {}, 0
    for words in lines:
        if words[0].endswith(LABEL_END):
            labels[words[0][:-1]] = pc
        else:
            pc += instruction_size(words[0])
    return labels


def encode_instruction(words: list[str], code: Code, labels: dict) -> bytes:
    name, dst = words[0], code.registers[words[1]]
    head = bytes([code.opcodes[name]])
    if name == "set":
        return head + bytes([dst]) + int(words[2]).to_bytes(IMMEDIATE_BYTES, "little")
    if name == "jz":
        return head + bytes([dst, labels[words[2]]])
    return head + bytes([dst | code.registers[words[2]] << REGISTER_SHIFT])


def code_mask(mask: tuple[int, int], pc: int) -> int:
    mult, add = mask
    return ((pc * mult + add) & WORD_MASK) >> CELL_MASK_SHIFT


def assemble(source: str, code: Code) -> bytes:
    lines = source_lines(source)
    labels = label_addresses(lines)
    plain = b"".join(encode_instruction(w, code, labels) for w in lines if not w[0].endswith(LABEL_END))
    if len(plain) > MAX_PROGRAM_LENGTH:
        raise ValueError(f"program is {len(plain)} bytes, max {MAX_PROGRAM_LENGTH}")
    return bytes(b ^ code_mask(code.mask, pc) for pc, b in enumerate(plain))


def shuffled_code(seed: int) -> Code:
    rng = random.Random(seed ^ SHUFFLE_SALT)
    opcodes = dict(zip(OPCODE_NAMES, rng.sample(range(BYTE_VALUES), len(OPCODE_NAMES))))
    registers = dict(zip(REGISTER_NAMES, rng.sample(range(REGISTER_COUNT), len(REGISTER_NAMES))))
    mask = (rng.getrandbits(WORD_BITS) | ODD, rng.getrandbits(WORD_BITS))
    return Code(opcodes, registers, mask, b"")


def build_machine(layout: Layout, seed: int) -> Machine:
    code = shuffled_code(seed)
    code.program = assemble(PROGRAM_SOURCE.format(**program_values(layout.size)), code)
    return Machine(code, [c ^ cell_mask(i) for i, c in enumerate(layout.cells)])


def build_maze(crackme: dict) -> tuple[Layout, Maze]:
    layout = build_layout(crackme)
    machine = build_machine(layout, crackme["maze_seed"])
    return layout, Maze(machine, layout.size, layout.start, layout.checkpoints)


def search(maze: Maze) -> tuple[dict, dict, dict]:
    first = (maze.start, 0)
    depth, ways, parent = {first: 0}, {first: 1}, {}
    queue = collections.deque([first])
    while queue:
        state = queue.popleft()
        for move in range(len(MOVES)):
            after = maze.step(*state, move)
            if after not in depth:
                depth[after], ways[after], parent[after] = depth[state] + 1, 0, (state, move)
                queue.append(after)
            if depth[after] == depth[state] + 1:
                ways[after] += ways[state]
    return depth, ways, parent


def goal_of(maze: Maze, depth: dict) -> tuple[int, int]:
    goals = [state for state in depth if state[1] == maze.checkpoints]
    if not goals:
        raise ValueError("the checkpoints can not all be reached")
    return min(goals, key=depth.get)


def solve(maze: Maze) -> tuple[str, int]:
    depth, ways, parent = search(maze)
    state = goal_of(maze, depth)
    count, path = ways[state], []
    while state in parent:
        state, move = parent[state]
        path.append(MOVES[move])
    return "".join(reversed(path)), count


def walk(maze: Maze, path: str) -> list[int]:
    cell, progress, words = maze.start, 0, []
    for letter in path:
        cell, progress, word = maze.machine.run(cell, progress, MOVES.index(letter))
        words.append(word)
    return words


def finished(maze: Maze, path: str) -> bool:
    return bool(path) and walk(maze, path)[-1] >> PROGRESS_SHIFT & BYTE_MASK == maze.checkpoints


def mix(value: int) -> int:
    value = ((value ^ (value >> MIX_SHIFT_A)) * MIX_A) & LONG_MASK
    value = ((value ^ (value >> MIX_SHIFT_B)) * MIX_B) & LONG_MASK
    return value ^ (value >> MIX_SHIFT_C)


def absorb(words: list[int]) -> int:
    state = START_STATE
    for word in words:
        state = mix(((state ^ word) + GOLDEN) & LONG_MASK)
    return state


def one_way(state: int, rounds: int) -> int:
    for _ in range(rounds):
        state = (state + GOLDEN) & LONG_MASK
        state ^= mix(state)
    return state


def slow_mix(state: int) -> int:
    return one_way(state, SLOW_ROUNDS)


def derive(maze: Maze, path: str) -> int:
    return slow_mix(absorb(walk(maze, path)))


def check_tag(value: int) -> int:
    return one_way(value ^ TAG_SALT, SEAL_ROUNDS)


def apply_keystream(data: bytes, value: int) -> bytes:
    key, stream = value ^ KEY_SALT, b""
    while len(stream) < len(data):
        key = (key + GOLDEN) & LONG_MASK
        stream += one_way(key, SEAL_ROUNDS).to_bytes(BLOCK_BYTES, "little")
    return bytes(a ^ b for a, b in zip(data, stream))


def cell_mask(cell: int) -> int:
    return ((cell * CELL_MASK_MULT + CELL_MASK_ADD) & WORD_MASK) >> CELL_MASK_SHIFT


def validate_layout(crackme: dict) -> None:
    size, points = crackme["maze_size"], [crackme["start"], *crackme["checkpoints"]]
    if not MIN_MAZE_SIZE <= size <= MAX_MAZE_SIZE:
        raise ValueError(f"maze_size must be {MIN_MAZE_SIZE} to {MAX_MAZE_SIZE}")
    if not 1 <= len(crackme["checkpoints"]) <= MAX_CHECKPOINTS:
        raise ValueError(f"need 1 to {MAX_CHECKPOINTS} checkpoints")
    if any(not (0 <= x < size and 0 <= y < size) for x, y in points):
        raise ValueError("start and checkpoints must be inside the maze")
    if len({tuple(p) for p in crackme["checkpoints"]}) != len(crackme["checkpoints"]):
        raise ValueError("checkpoints must be different squares")


def validate_reveal(reveal: str) -> None:
    if not MIN_REVEAL_LENGTH <= len(reveal) <= MAX_REVEAL_LENGTH:
        raise ValueError(f"reveal must be {MIN_REVEAL_LENGTH} to {MAX_REVEAL_LENGTH} chars")
    if any(not FIRST_PRINTABLE <= ord(c) <= LAST_PRINTABLE for c in reveal):
        raise ValueError("reveal must be printable ASCII")


def validate_path(path: str, count: int) -> None:
    if count != 1:
        raise ValueError(f"{count} shortest paths, change maze_seed or the checkpoints")
    if not MIN_PATH_LENGTH <= len(path) <= MAX_PATH_LENGTH:
        raise ValueError(f"path is {len(path)} keys, must be {MIN_PATH_LENGTH} to {MAX_PATH_LENGTH}")


def reference_step(layout: Layout, cell: int, progress: int, move: int) -> tuple[int, int, int]:
    if move == PRESS:
        progress += layout.cells[cell] >> MARK_SHIFT == progress + 1
    elif not layout.cells[cell] & (1 << move):
        cell = neighbor(layout.size, cell, move)
    return cell, progress, (move << MOVE_SHIFT) | (progress << PROGRESS_SHIFT) | cell


def reference_walk(layout: Layout, path: str) -> list[int]:
    cell, progress, words = layout.start, 0, []
    for letter in path:
        cell, progress, word = reference_step(layout, cell, progress, MOVES.index(letter))
        words.append(word)
    return words


def every_case(layout: Layout) -> list[tuple[int, int, int]]:
    return [(cell, progress, move) for cell in range(len(layout.cells))
            for progress in range(layout.checkpoints + 1) for move in range(len(MOVES))]


def check_machine(layout: Layout, machine: Machine) -> int:
    cases = every_case(layout)
    for case in cases:
        got, want = machine.run(*case), reference_step(layout, *case)
        if got != want:
            raise AssertionError(f"machine gives {got} for {case}, reference gives {want}")
    return len(cases)


def checked_path(crackme: dict) -> tuple[Maze, str]:
    validate_layout(crackme)
    validate_reveal(crackme["reveal"])
    layout, maze = build_maze(crackme)
    print(f"crackme: machine matches reference on {check_machine(layout, maze.machine)} cases")
    path, count = solve(maze)
    validate_path(path, count)
    if walk(maze, path) != reference_walk(layout, path):
        raise AssertionError("machine walk differs from reference walk")
    return maze, path


def c_array(c_type: str, name: str, length: str, values: list[str]) -> str:
    rows = [", ".join(values[i:i + 16]) for i in range(0, len(values), 16)]
    return f"static const {c_type} {name}[{length}] = {{\n    " + ",\n    ".join(rows) + ",\n};\n"


def hex_bytes(data) -> list[str]:
    return [f"0x{b:02x}" for b in data]


def code_defines(code: Code) -> str:
    opcodes = "".join(f"#define OP_{name.upper()} 0x{n:02x}\n" for name, n in code.opcodes.items())
    registers = "".join(f"#define {name.upper()}_REGISTER {code.registers[name]}\n"
                        for name in SHARED_REGISTERS)
    mult, add = code.mask
    return (opcodes + registers
            + f"#define CODE_MASK_MULT 0x{mult:08x}u\n#define CODE_MASK_ADD 0x{add:08x}u\n"
            + f"#define CODE_MASK_SHIFT {CELL_MASK_SHIFT}\n#define PROGRAM_LENGTH {len(code.program)}\n")


def header_text(maze: Maze, path: str, reveal: str) -> str:
    value, code = derive(maze, path), maze.machine.code
    secret = hex_bytes(apply_keystream(reveal.encode("ascii"), value))
    return (
        "#pragma once\n\n/* generated by hunt/crackme_secret.py, do not edit */\n\n#include <stdint.h>\n\n"
        f"#define MAZE_SIZE {maze.size}\n#define CELL_COUNT {len(maze.machine.table)}\n"
        f"#define START_CELL {maze.start}\n#define CHECKPOINT_COUNT {maze.checkpoints}\n"
        f"#define SECRET_LENGTH {len(secret)}\n#define CHECK_TAG 0x{check_tag(value):016x}ull\n\n"
        + code_defines(code) + "\n"
        + c_array("uint8_t", "MAZE_CELLS", "CELL_COUNT", hex_bytes(maze.machine.table)) + "\n"
        + c_array("uint8_t", "PROGRAM", "PROGRAM_LENGTH", hex_bytes(code.program)) + "\n"
        + c_array("uint8_t", "ENCRYPTED_SECRET", "SECRET_LENGTH", secret)
    )


def write_header(crackme: dict, path: pathlib.Path) -> None:
    maze, keys = checked_path(crackme)
    path.write_text(header_text(maze, keys, crackme["reveal"]))
    print(f"crackme: {len(keys)} keys {keys}")


def read_array(text: str, name: str) -> list[int]:
    body = re.search(name + r"\[\w+\] = \{(.*?)\};", text, re.S).group(1)
    return [int(v.strip(), 16) for v in body.split(",") if v.strip()]


def read_define(text: str, name: str) -> int:
    return int(re.search(rf"#define {name} (\w+?)u?l?l?\n", text).group(1), 0)


def device_code(text: str) -> Code:
    opcodes = {name: read_define(text, f"OP_{name.upper()}") for name in OPCODE_NAMES}
    registers = {name: read_define(text, f"{name.upper()}_REGISTER") for name in SHARED_REGISTERS}
    mask = (read_define(text, "CODE_MASK_MULT"), read_define(text, "CODE_MASK_ADD"))
    return Code(opcodes, registers, mask, bytes(read_array(text, "PROGRAM")))


def device_maze(text: str) -> Maze:
    machine = Machine(device_code(text), read_array(text, "MAZE_CELLS"))
    return Maze(machine, read_define(text, "MAZE_SIZE"), read_define(text, "START_CELL"),
                read_define(text, "CHECKPOINT_COUNT"))


def device_unlock(text: str, path: str) -> str | None:
    value = derive(device_maze(text), path)
    if check_tag(value) != read_define(text, "CHECK_TAG"):
        return None
    return apply_keystream(bytes(read_array(text, "ENCRYPTED_SECRET")), value).decode("ascii")


def assert_catches_broken_program() -> None:
    layout, maze = build_maze(SAMPLE)
    program = bytearray(maze.machine.code.program)
    program[-1] ^= 1
    maze.machine.code.program = bytes(program)
    try:
        check_machine(layout, maze.machine)
    except AssertionError as error:
        print(f"broken program caught: {error}")
        return
    raise AssertionError("broken program passed the equivalence test")


def assert_rejects(crackme: dict) -> None:
    try:
        checked_path(crackme)
    except ValueError as error:
        print(f"rejected: {error}")
        return
    raise AssertionError(f"accepted bad config {crackme!r}")


def assert_rejects_bad_input() -> None:
    assert_rejects(OPEN_GRID)
    assert_rejects({**SAMPLE, "reveal": "short"})
    assert_rejects({**SAMPLE, "reveal": "tab\there!"})
    assert_rejects({**SAMPLE, "checkpoints": [[8, 0]]})
    assert_rejects({**SAMPLE, "checkpoints": [[1, 0]]})


def selftest() -> None:
    maze, path = checked_path(SAMPLE)
    assert finished(maze, path) and not finished(maze, path[:-1])
    print(f"path {path!r} ({len(path)} keys) solves the maze, unique shortest")
    with tempfile.TemporaryDirectory() as tmp:
        header = pathlib.Path(tmp) / "crackme_secret.h"
        write_header(SAMPLE, header)
        text = header.read_text()
    assert SAMPLE["reveal"].encode() not in text.encode()
    device = device_maze(text).machine
    assert device.table == maze.machine.table and device.code.program == maze.machine.code.program
    assert device_unlock(text, path) == SAMPLE["reveal"]
    print(f"pass  {path!r} -> {SAMPLE['reveal']!r}")
    for wrong in (*WRONG_PATHS, path[:-1], path + "U"):
        assert device_unlock(text, wrong) is None
        print(f"fail  {wrong!r}")
    assert_catches_broken_program()
    assert_rejects_bad_input()
    print("selftest ok")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--selftest", action="store_true")
    if parser.parse_args().selftest:
        selftest()
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
