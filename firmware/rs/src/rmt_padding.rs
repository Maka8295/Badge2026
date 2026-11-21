use esp_idf_svc::hal::rmt::{PinState, Pulse, PulseTicks, Symbol};

const PING_PONG_HALVES: usize = 2;
const IDLE_TICKS: u16 = 1;

/// esp-idf-hal 0.47 panics inside the RMT interrupt when the copy encoder
/// fills the channel memory exactly on its last chunk. One extra idle symbol
/// moves the end off that boundary.
pub fn avoid_exact_fill(symbols: &mut Vec<Symbol>, memory_block_symbols: usize) {
    if symbols.len() % (memory_block_symbols / PING_PONG_HALVES) == 0 {
        let idle = Pulse::new(PinState::Low, PulseTicks::new(IDLE_TICKS).unwrap());
        symbols.push(Symbol::new(idle, idle));
    }
}
