#![no_std]
#![no_main]

use panic_halt as _;
// use panic_itm as _; // logs messages over ITM; requires ITM support

use cortex_m::asm;
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    asm::nop(); // To not have main optimize to abort in release mode, remove when you add code

    loop {
        // your code goes here
    }
}
