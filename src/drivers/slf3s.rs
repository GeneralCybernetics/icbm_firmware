use embassy_stm32::gpio::{Level, Output, Pin, Speed};
use embassy_stm32::i2c::I2c;
use embassy_stm32::mode::Async;
use embassy_stm32::Peripheral;
use embassy_time::Timer;

// Constants for scale factors
const SLF3X_SCALE_FACTOR_FLOW: f32 = 500.0;
const SLF3X_SCALE_FACTOR_TEMP: f32 = 200.0;

// I2C Address
const SLF3X_I2C_ADDRESS: u8 = 0x08;

// Continuous Command
const CMD_START_MEASUREMENT_LENGTH: usize = 2;
const CMD_START_MEASUREMENT: [u8; CMD_START_MEASUREMENT_LENGTH] = [0x36, 0x08];
const DATA_LENGTH: usize = 9;
const INITIAL_MEASURE_DELAY: u64 = 50; // Milliseconds

// Stop measurement Command
const CMD_STOP_MEASUREMENT: [u8; 2] = [0x3F, 0xF9];

// Soft reset Command
const SOFT_RESET_I2C_ADDRESS: u8 = 0x00;
const CMD_SOFT_RESET_LENGTH: usize = 1;
const CMD_SOFT_RESET: [u8; CMD_SOFT_RESET_LENGTH] = [0x06];
const CHIP_RESET_DELAY: u64 = 50; // Milliseconds

//Address Change Command
const CMD_ADDR_CHANGE_LENGHT: usize = 2;
const CMD_ADDR_CHANGE: [u8; CMD_ADDR_CHANGE_LENGHT] = [0x36, 0x61];

pub struct SLF3S<'d> {
    i2c: I2c<'d, Async>,
    flow_scale_factor: f32,
    temp_scale_factor: f32,
    i2c_address: u8,
}

impl<'d> SLF3S<'d> {
    pub fn new(i2c: I2c<'d, Async>) -> Self {
        Self {
            i2c,
            flow_scale_factor: SLF3X_SCALE_FACTOR_FLOW,
            temp_scale_factor: SLF3X_SCALE_FACTOR_TEMP,
            i2c_address: SLF3X_I2C_ADDRESS,
        }
    }

    pub async fn start_measurement(&mut self) -> Result<(), &'static str> {
        self.i2c
            .write(self.i2c_address, &CMD_START_MEASUREMENT)
            .await
            .map_err(|_| "Failed to start measurement")?;
        Timer::after_millis(INITIAL_MEASURE_DELAY.into()).await;
        Ok(())
    }

    pub async fn stop_measurement(&mut self) -> Result<(), &'static str> {
        self.i2c
            .write(self.i2c_address, &CMD_STOP_MEASUREMENT)
            .await
            .map_err(|_| "Failed to stop measurement")?;
        Ok(())
    }

    pub async fn read_sample(&mut self) -> Result<(f32, f32), &'static str> {
        let mut data = [0u8; DATA_LENGTH];
        self.i2c
            .read(self.i2c_address, &mut data)
            .await
            .map_err(|_| "Failed to read data")?;
        let flow = self.convert_and_scale(data[0], data[1], self.flow_scale_factor);
        let temp = self.convert_and_scale(data[3], data[4], self.temp_scale_factor);
        Ok((flow, temp))
    }

    fn convert_and_scale(&self, b1: u8, b2: u8, scale_factor: f32) -> f32 {
        let value = i16::from_be_bytes([b1, b2]);
        value as f32 / scale_factor
    }

    pub async fn reset(&mut self) -> Result<(), &'static str> {
        self.i2c
            .write(SOFT_RESET_I2C_ADDRESS, &CMD_SOFT_RESET)
            .await
            .map_err(|_| "Failed to reset")?;
        Timer::after_millis(CHIP_RESET_DELAY.into()).await;
        Ok(())
    }

    pub async fn change_addr<P: Pin>(
        &mut self,
        new_addr: u16,
        pin: impl Peripheral<P = P> + 'd,
    ) -> Result<(), &'static str> {
        let mut gpio_pin = Output::new(pin, Level::Low, Speed::Low);

        self.reset().await?;

        let mut command = [0u8; 5];
        command[0..2].copy_from_slice(&CMD_ADDR_CHANGE);
        command[2..4].copy_from_slice(&new_addr.to_be_bytes());
        command[4] = Self::crc8(&command[2..4]); // Calculate CRC for the new address bytes

        // Send the command to change the address
        self.i2c
            .write(self.i2c_address, &command)
            .await
            .map_err(|_| "Failed to send address change command")?;

        Timer::after_micros(100).await;

        gpio_pin.set_high();
        Timer::after_micros(300).await;
        gpio_pin.set_low();

        // Wait for the 1.5ms monitoring process to complete
        Timer::after_millis(2).await;

        // The device should now respond to the new address
        self.i2c_address = (new_addr & 0xFF) as u8;

        // Wait for the confirmation pulse (200us high)
        Timer::after_micros(200).await;

        Ok(())
    }

    pub fn rtrn_addr(&mut self) -> u8 {
        self.i2c_address
    }

    fn crc8(data: &[u8]) -> u8 {
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
}
