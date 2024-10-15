#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::Output;
use embassy_stm32::i2c::{Config, I2c};
use embassy_stm32::{bind_interrupts, i2c, peripherals};

use embassy_stm32::time::Hertz;
use embassy_time::{block_for, Duration, Timer};
use {defmt_rtt as _, panic_probe as _};
mod drivers;
use drivers::scd41::SCD41;

bind_interrupts!(struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    let mut config = Config::default();
    config.scl_pullup = true;
    config.sda_pullup = true;

    let mut i2c = I2c::new(
        p.I2C1,
        p.PB6,
        p.PB7,
        Irqs,
        p.DMA1_CH6,
        p.DMA1_CH0,
        Hertz(100_000),
        config,
    );

    // match i2c.write(0x62, &[0x21, 0xB1]).await {
    //     Ok(()) => {
    //         info!("works");
    //     }
    //     Err(e) => {
    //         error!("error {}", e);
    //     }
    // }

    let mut scd41sensor = SCD41::new(i2c);

    match scd41sensor.stop_periodic_measurement().await {
        Ok(()) => {
            info!("works!");
        }
        Err(e) => {
            error!("error: {}", e)
        }
    }

    Timer::after_millis(500).await;

    match scd41sensor.init().await {
        Ok(()) => {
            info!("works!");
        }
        Err(e) => {
            error!("error: {}", e)
        }
    }

    // match i2c.write(0x62, &[0x36, 0x39]).await {
    //     Ok(()) => {
    //         info!("Self-test command sent successfully")
    //     }
    //     Err(e) => error!("Error sending self-test command: {}", e),
    // }

    // // Wait for the specified execution time (10 seconds)
    // Timer::after_millis(10_000).await;

    // // Read the response (3 bytes)
    // let mut buf = [0u8; 3];
    // match i2c.read(0x62, &mut buf).await {
    //     Ok(()) => {
    //         info!(
    //             "Self-test response read successfully: {:#04x} {:#04x} {:#04x}",
    //             buf[0], buf[1], buf[2]
    //         );

    //         // Interpret the result
    //         if buf[0] == 0x00 && buf[1] == 0x00 {
    //             info!("Self-test passed: No malfunction detected");
    //         } else {
    //             error!("Self-test failed: Malfunction detected");
    //         }
    //     }
    //     Err(e) => error!("Error reading self-test response: {}", e),
    // }

    // let mut scd_sensor = SCD41::new(i2c);

    // match scd_sensor.init().await {
    //     Ok(()) => {}
    //     Err(e) => {
    //         error!("{}", e)
    //     }
    // }

    // match scd_sensor.start_periodic_measurement().await {
    //     Ok(()) => {}
    //     Err(e) => {
    //         error!("{}", e)
    //     }
    // }
}

pub fn crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0xFF; // Initialization value
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ 0x31; // 0x31 is the polynomial
            } else {
                crc <<= 1;
            }
        }
    }
    crc // No final XOR needed as it's 0x00
}
