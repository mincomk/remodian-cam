use esp_hal::{
    Async,
    gpio::Level,
    rmt::{Channel, Error, PulseCode, Tx},
};
use heapless::Vec;

// RC5 half-bit period in microseconds (RMT ticks at 1µs/tick with clk_divider=80)
const HALF_BIT: u16 = 889;

/// Send an RC5 IR command.
///
/// RC5 protocol: 14-bit Manchester-encoded frame transmitted at 36 kHz carrier.
/// Frame: S1=1, S2=1, Toggle, A4..A0 (5-bit address), C5..C0 (6-bit command)
pub async fn send_rc5(
    channel: &mut Channel<'_, Async, Tx>,
    address: u8,
    command: u8,
    toggle: bool,
) -> Result<(), Error> {
    // 14 bits + 1 end marker
    let mut pulses: Vec<PulseCode, 15> = Vec::new();

    // Build 14-bit word: [S1=1][S2=1][T][A4..A0][C5..C0]
    let mut data: u16 = 0b11 << 12;
    if toggle {
        data |= 1 << 11;
    }
    data |= ((address & 0x1F) as u16) << 6;
    data |= (command & 0x3F) as u16;

    for i in (0..14).rev() {
        let bit = (data >> i) & 1;
        // Manchester encoding:
        //   1 → first half LOW (space), second half HIGH (mark)
        //   0 → first half HIGH (mark), second half LOW (space)
        let pulse = if bit == 1 {
            PulseCode::new(Level::Low, HALF_BIT, Level::High, HALF_BIT)
        } else {
            PulseCode::new(Level::High, HALF_BIT, Level::Low, HALF_BIT)
        };
        let _ = pulses.push(pulse);
    }
    let _ = pulses.push(PulseCode::end_marker());

    channel.transmit(pulses.as_slice()).await
}

/// Send an RC5-Extended (RC5X) IR command.
///
/// RC5X extends RC5 to 7-bit commands by using the S2 bit as the inverted
/// 7th command bit (C6). Commands 0–63 behave identically to RC5 (S2=1).
/// Commands 64–127 set S2=0, encoding C6=1 implicitly.
pub async fn send_rc5x(
    channel: &mut Channel<'_, Async, Tx>,
    address: u8,
    command: u8,
    toggle: bool,
) -> Result<(), Error> {
    let mut pulses: Vec<PulseCode, 15> = Vec::new();

    // S2 is the inverted 7th command bit (C6)
    let s2 = (command >> 6) & 1 == 0;

    // Build 14-bit word: [S1=1][S2=~C6][T][A4..A0][C5..C0]
    let mut data: u16 = 1 << 13; // S1 always 1
    if s2 {
        data |= 1 << 12;
    }
    if toggle {
        data |= 1 << 11;
    }
    data |= ((address & 0x1F) as u16) << 6;
    data |= (command & 0x3F) as u16;

    for i in (0..14).rev() {
        let bit = (data >> i) & 1;
        let pulse = if bit == 1 {
            PulseCode::new(Level::Low, HALF_BIT, Level::High, HALF_BIT)
        } else {
            PulseCode::new(Level::High, HALF_BIT, Level::Low, HALF_BIT)
        };
        let _ = pulses.push(pulse);
    }
    let _ = pulses.push(PulseCode::end_marker());

    channel.transmit(pulses.as_slice()).await
}
