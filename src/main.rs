#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use icbm_firmware::drivers::co2_solenoid;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
}
