pub type Key = (u8, u8);

pub const ESC: Key = (1, 1);
pub const DEL: Key = (1, 14);
pub const TAB: Key = (2, 1);
pub const SHIFT: Key = (3, 2);
pub const OK: Key = (3, 14);
pub const DOWN: Key = (5, 1);
pub const LEFT: Key = (5, 2);
pub const UP: Key = (6, 1);
pub const A: Key = (6, 2);
pub const RIGHT: Key = (7, 1);
pub const B: Key = (7, 2);

use super::matrix::COLS;

type Legend = Option<(char, char)>;

const SPECIAL: Legend = None;

const fn legend(plain: char, shifted: char) -> Legend {
    Some((plain, shifted))
}

const ROWS_WITH_LEGENDS: usize = 4;

const LEGENDS: [[Legend; COLS]; ROWS_WITH_LEGENDS] = [
    [SPECIAL, legend('1', '!'), legend('2', '@'), legend('3', '#'), legend('4', '$'), legend('5', '%'),
     legend('6', '^'), legend('7', '&'), legend('8', '*'), legend('9', '('), legend('0', ')'),
     legend('-', '_'), legend('=', '+'), SPECIAL],
    [SPECIAL, legend('q', 'Q'), legend('w', 'W'), legend('e', 'E'), legend('r', 'R'), legend('t', 'T'),
     legend('y', 'Y'), legend('u', 'U'), legend('i', 'I'), legend('o', 'O'), legend('p', 'P'),
     legend('[', '{'), legend(']', '}'), legend('\\', '|')],
    [SPECIAL, SPECIAL, legend('a', 'A'), legend('s', 'S'), legend('d', 'D'), legend('f', 'F'),
     legend('g', 'G'), legend('h', 'H'), legend('j', 'J'), legend('k', 'K'), legend('l', 'L'),
     legend(';', ':'), legend('\'', '"'), SPECIAL],
    [SPECIAL, SPECIAL, SPECIAL, legend('z', 'Z'), legend('x', 'X'), legend('c', 'C'),
     legend('v', 'V'), legend('b', 'B'), legend('n', 'N'), legend('m', 'M'), legend(',', '<'),
     legend('.', '>'), legend('/', '?'), legend(' ', ' ')],
];

pub fn char_for(k: Key, shift: bool) -> Option<char> {
    let (row, col) = k;
    let row_legends = LEGENDS.get(usize::from(row).checked_sub(1)?)?;
    let (plain, shifted) = row_legends[usize::from(col) - 1]?;
    Some(if shift { shifted } else { plain })
}
