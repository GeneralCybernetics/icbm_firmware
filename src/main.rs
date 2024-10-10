#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::usart::{Config, DataBits, Parity, StopBits, Uart};
use embassy_stm32::{bind_interrupts, peripherals, usart};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};
mod drivers;
use drivers::explorir_m_e_100::ExplorIrME100;

bind_interrupts!(struct Irqs {
    USART3 => usart::InterruptHandler<peripherals::USART3>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Hello World!");

    let mut config = Config::default();
    config.baudrate = 9600;
    config.parity = Parity::ParityNone;
    config.stop_bits = StopBits::STOP1;
    config.data_bits = DataBits::DataBits8;

    let mut usart =
        Uart::new(p.USART3, p.PD9, p.PD8, Irqs, p.DMA1_CH3, p.DMA1_CH1, config).unwrap();

    Timer::after_secs(2).await;

    // let cmd = b".\r\n"; //this is the command to read the serial no

    // match usart.write(cmd).await {
    //     Ok(_) => {
    //         info!("Command sent successfully");
    //     }
    //     Err(_) => {
    //         error!("Failed to send command");
    //     }
    // }

    // let mut response = [0u8; 47];
    // let mut index = 0;

    // while index < response.len() {
    //     match usart.read(&mut response[index..index + 1]).await {
    //         Ok(_) => {
    //             index += 1;
    //         }
    //         Err(_) => {
    //             error!("Error while reading");
    //             break;
    //         }
    //     }
    // }

    // info!(
    //     "Raw response bytes: {:?}",
    //     core::str::from_utf8(&response[..index]).unwrap_or("<invalid UTF-8>")
    // );
    let mut co2_sensor = ExplorIrME100::new(usart);
    match co2_sensor.get_filtered_co2().await {
        Ok(co2_level) => {
            info!("CO2 level: {} ppm", co2_level);
        }
        Err(error_msg) => {
            info!("Failed to get CO2 reading: {}", error_msg);
        }
    }

    match co2_sensor.get_unfiltered_co2().await {
        Ok(co2_level) => {
            info!("CO2 level: {} ppm", co2_level);
        }
        Err(error_msg) => {
            info!("Failed to get CO2 reading: {}", error_msg);
        }
    }
}
