#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::usart::{Config, DataBits, Parity, StopBits, Uart};
use embassy_stm32::{bind_interrupts, peripherals, usart};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USART3 => usart::InterruptHandler<peripherals::USART3>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Hello World!");

    let mut config = Config::default();
    config.baudrate = 19200;
    config.parity = Parity::ParityNone;
    config.stop_bits = StopBits::STOP1;
    config.data_bits = DataBits::DataBits8;

    let mut usart =
        Uart::new(p.USART3, p.PD9, p.PD8, Irqs, p.DMA1_CH3, p.DMA1_CH1, config).unwrap();

    let cmd = [0xFF, 0xFE, 0x02, 0x02, 0x01]; //this is the command to read the serial no

    match usart.write(&cmd).await {
        Ok(_) => {
            info!("Command sent successfully");
        }
        Err(_) => {
            error!("Failed to send command");
        }
    }

    let mut response = [0u8; 20];
    let mut index = 0;
    while index < response.len() {
        match usart.read(&mut response[index..index + 1]).await {
            Ok(_) => {
                index += 1;
                if index >= 3 && response[2] as usize + 3 == index {
                    break;
                }
            }
            Err(_) => error!("error while reading"),
        }
    }

    if index >= 6 && response[0] == 0xFF && response[1] == 0xFA {
        let mut serial_number = [0u8; 15];
        serial_number.copy_from_slice(&response[3..18]);
        info!("{:02X}", serial_number)
    } else {
        error!("error displaying the serial number")
    }
}
