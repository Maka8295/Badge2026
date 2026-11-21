#pragma once

#include <stdint.h>

#include "crackme_secret.h"

#define REGISTER_COUNT 8
#define REGISTER_MASK 7u
#define REGISTER_SHIFT 4
#define HEAD_BYTES 2
#define IMMEDIATE_BYTES 4
#define TARGET_BYTES 1
#define SHIFT_MASK 31u
#define BYTE_BITS 8

struct machine {
    uint32_t reg[REGISTER_COUNT];
    int pc;
};

static uint8_t code_at(int pc)
{
    uint32_t mask = ((uint32_t)pc * CODE_MASK_MULT + CODE_MASK_ADD) >> CODE_MASK_SHIFT;
    return PROGRAM[pc] ^ (uint8_t)mask;
}

static uint32_t immediate_at(int pc)
{
    uint32_t value = 0;
    for (int i = IMMEDIATE_BYTES - 1; i >= 0; i--) {
        value = (value << BYTE_BITS) | code_at(pc + i);
    }
    return value;
}

static uint32_t combine(int op, uint32_t a, uint32_t b)
{
    switch (op) {
    case OP_ADD: return a + b;
    case OP_MUL: return a * b;
    case OP_XOR: return a ^ b;
    case OP_AND: return a & b;
    case OP_SHR: return a >> (b & SHIFT_MASK);
    case OP_LOAD: return MAZE_CELLS[b % CELL_COUNT];
    default: return a;
    }
}

static void execute(struct machine *vm)
{
    int op = code_at(vm->pc);
    int regs = code_at(vm->pc + 1);
    uint32_t *dst = &vm->reg[regs & REGISTER_MASK];
    uint32_t src = vm->reg[(regs >> REGISTER_SHIFT) & REGISTER_MASK];
    vm->pc += HEAD_BYTES;
    if (op == OP_SET) {
        *dst = immediate_at(vm->pc);
        vm->pc += IMMEDIATE_BYTES;
    } else if (op == OP_JZ) {
        vm->pc = *dst ? vm->pc + TARGET_BYTES : code_at(vm->pc);
    } else {
        *dst = combine(op, *dst, src);
    }
}

static void run_machine(struct machine *vm)
{
    vm->pc = 0;
    while (vm->pc < PROGRAM_LENGTH) {
        execute(vm);
    }
}
