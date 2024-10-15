use defmt::{error, info};
use embassy_stm32::i2c::I2c;
use embassy_stm32::mode::Async;
use embassy_time::Timer;

// I2C Address
const SCD41_I2C_ADDRESS: u8 = 0x62;

const POWERUP_TIME: u32 = 40; // millisec

// Basic commands
const CMD_START_PERIODIC_MEASUREMENT: [u8; 2] = [0x21, 0xB1];
const CMD_READ_MEASUREMENT: [u8; 2] = [0xEC, 0x05];
const CMD_STOP_PERIODIC_MEASUREMENT: [u8; 2] = [0x3F, 0x86];

// On-chip output signal compensation
const CMD_SET_TEMPERATURE_OFFSET: [u8; 2] = [0x24, 0x1D];
const CMD_GET_TEMPERATURE_OFFSET: [u8; 2] = [0x23, 0x18];
const CMD_SET_SENSOR_ALTITUDE: [u8; 2] = [0x24, 0x27];
const CMD_GET_SENSOR_ALTITUDE: [u8; 2] = [0x23, 0x22];
const CMD_SET_AMBIENT_PRESSURE: [u8; 2] = [0xE0, 0x00];
const CMD_GET_AMBIENT_PRESSURE: [u8; 2] = [0xE0, 0x00];

// Field calibration
// const CMD_PERFORM_FORCED_RECALIBRATION: [u8; 2] = [0x36, 0x2F];
// const CMD_SET_AUTOMATIC_SELF_CALIBRATION_ENABLED: [u8; 2] = [0x24, 0x16];
// const CMD_GET_AUTOMATIC_SELF_CALIBRATION_ENABLED: [u8; 2] = [0x23, 0x13];
// const CMD_SET_AUTOMATIC_SELF_CALIBRATION_TARGET: [u8; 2] = [0x24, 0x3A];
// const CMD_GET_AUTOMATIC_SELF_CALIBRATION_TARGET: [u8; 2] = [0x23, 0x3F];

// Low power periodic measurement mode
// const CMD_START_LOW_POWER_PERIODIC_MEASUREMENT: [u8; 2] = [0x21, 0xAC];
const CMD_GET_DATA_READY_STATUS: [u8; 2] = [0xE4, 0xB8];

// Advanced features
// const CMD_PERSIST_SETTINGS: [u8; 2] = [0x36, 0x15];
// const CMD_GET_SERIAL_NUMBER: [u8; 2] = [0x36, 0x82];
const CMD_PERFORM_SELF_TEST: [u8; 2] = [0x36, 0x39];
// const CMD_PERFORM_FACTORY_RESET: [u8; 2] = [0x36, 0x32];
// const CMD_REINIT: [u8; 2] = [0x36, 0x46];
// const CMD_GET_SENSOR_VARIANT: [u8; 2] = [0x20, 0x2F];

// Single shot measurement mode
// const CMD_MEASURE_SINGLE_SHOT: [u8; 2] = [0x21, 0x9D];
// const CMD_MEASURE_SINGLE_SHOT_RHT_ONLY: [u8; 2] = [0x21, 0x96];
// const CMD_POWER_DOWN: [u8; 2] = [0x36, 0xE0];
// const CMD_WAKE_UP: [u8; 2] = [0x36, 0xF6];
// const CMD_SET_AUTOMATIC_SELF_CALIBRATION_INITIAL_PERIOD: [u8; 2] = [0x24, 0x45];
// const CMD_GET_AUTOMATIC_SELF_CALIBRATION_INITIAL_PERIOD: [u8; 2] = [0x23, 0x40];
// const CMD_SET_AUTOMATIC_SELF_CALIBRATION_STANDARD_PERIOD: [u8; 2] = [0x24, 0x4E];
// const CMD_GET_AUTOMATIC_SELF_CALIBRATION_STANDARD_PERIOD: [u8; 2] = [0x23, 0x4B];

// Execution times (in milliseconds)
const INITIAL_MEASURE_DELAY: u32 = 500;
const STOP_MEASURE_DELAY: u32 = 500;
const PERFORM_SELF_TEST_DELAY: u32 = 10_000;

// const EXECUTION_TIME_READ_MEASUREMENT: u32 = 1;
// const EXECUTION_TIME_STOP_PERIODIC_MEASUREMENT: u32 = 500;
// const EXECUTION_TIME_SET_TEMPERATURE_OFFSET: u32 = 1;
// const EXECUTION_TIME_GET_TEMPERATURE_OFFSET: u32 = 1;
// const EXECUTION_TIME_SET_SENSOR_ALTITUDE: u32 = 1;
// const EXECUTION_TIME_GET_SENSOR_ALTITUDE: u32 = 1;
// const EXECUTION_TIME_SET_AMBIENT_PRESSURE: u32 = 1;
// const EXECUTION_TIME_GET_AMBIENT_PRESSURE: u32 = 1;
// const EXECUTION_TIME_PERFORM_FORCED_RECALIBRATION: u32 = 400;
// const EXECUTION_TIME_SET_AUTOMATIC_SELF_CALIBRATION_ENABLED: u32 = 1;
// const EXECUTION_TIME_GET_AUTOMATIC_SELF_CALIBRATION_ENABLED: u32 = 1;
// const EXECUTION_TIME_SET_AUTOMATIC_SELF_CALIBRATION_TARGET: u32 = 1;
// const EXECUTION_TIME_GET_AUTOMATIC_SELF_CALIBRATION_TARGET: u32 = 1;
// const EXECUTION_TIME_GET_DATA_READY_STATUS: u32 = 1;
// const EXECUTION_TIME_PERSIST_SETTINGS: u32 = 800;
// const EXECUTION_TIME_GET_SERIAL_NUMBER: u32 = 1;
// const EXECUTION_TIME_PERFORM_SELF_TEST: u32 = 10000;
// const EXECUTION_TIME_PERFORM_FACTORY_RESET: u32 = 1200;
// const EXECUTION_TIME_REINIT: u32 = 30;
// const EXECUTION_TIME_GET_SENSOR_VARIANT: u32 = 1;
// const EXECUTION_TIME_MEASURE_SINGLE_SHOT: u32 = 5000;
// const EXECUTION_TIME_MEASURE_SINGLE_SHOT_RHT_ONLY: u32 = 50;
// const EXECUTION_TIME_POWER_DOWN: u32 = 1;
// const EXECUTION_TIME_WAKE_UP: u32 = 30;
// const EXECUTION_TIME_SET_AUTOMATIC_SELF_CALIBRATION_INITIAL_PERIOD: u32 = 1;
// const EXECUTION_TIME_GET_AUTOMATIC_SELF_CALIBRATION_INITIAL_PERIOD: u32 = 1;
// const EXECUTION_TIME_SET_AUTOMATIC_SELF_CALIBRATION_STANDARD_PERIOD: u32 = 1;
// const EXECUTION_TIME_GET_AUTOMATIC_SELF_CALIBRATION_STANDARD_PERIOD: u32 = 1;

//TODO make sure you stop measurement before any init commands
pub struct SCD41<'d> {
    i2c: I2c<'d, Async>,
    i2c_address: u8,
}

impl<'d> SCD41<'d> {
    pub fn new(i2c: I2c<'d, Async>) -> Self {
        SCD41 {
            i2c,
            i2c_address: SCD41_I2C_ADDRESS,
        }
    }

    pub async fn init(&mut self) -> Result<(), &'static str> {
        match self.perform_self_test().await {
            Ok(true) => {
                info!("SCD41 self-test passed");
                Ok(())
            }
            Ok(false) => {
                error!("Sensor malfunction detected -- this could be either physical or temporary");
                Err("Sensor malfunction detected")
            }
            Err(e) => {
                error!("Error while trying to perform self test: {}", e);
                Err(e)
            }
        }
        // [TODO] Set offsets here!!
    }

    //command words are not followed by CRC!
    pub fn crc8(&mut self, data: &[u8]) -> u8 {
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

    async fn send_command(&mut self, address: &[u8]) -> Result<(), &'static str> {
        self.i2c
            .write(self.i2c_address, address)
            .await
            .map_err(|_| "Error writing command")
    }

    async fn write_command(&mut self, address: &[u8], data: &[u8]) -> Result<(), &'static str> {
        const MAX_LENGTH: usize = 5; // 2 (addr) + 2 (data) + 1(crc)
        let mut combined = [0u8; MAX_LENGTH];

        if address.len() + data.len() + 1 > MAX_LENGTH {
            return Err("Command too long");
        }

        combined[..address.len()].copy_from_slice(address);
        combined[address.len()..address.len() + data.len()].copy_from_slice(data);
        combined[address.len() + data.len()] = self.crc8(data);

        self.i2c
            .write(self.i2c_address, &combined)
            .await
            .map_err(|_| "Error writing command")
    }

    //make sure SR here is same as ST according to the transaction contract
    //i.e make sure that both write_read and write and then read works here [TODO]
    async fn read_sequence(&mut self, address: &[u8], buf: &mut [u8]) -> Result<(), &'static str> {
        self.i2c
            .write_read(self.i2c_address, address, buf)
            .await
            .map_err(|_| "Error reading sequence")?;

        for chunk in buf.chunks(3) {
            if chunk.len() == 3 {
                let data = &chunk[0..2];
                let received_crc = chunk[2];
                let calculated_crc = self.crc8(data);
                if calculated_crc != received_crc {
                    return Err("CRC mismatch in read data");
                }
            } else {
                return Err("The chunck was not divisible by 3");
            }
        }

        Ok(())
    }

    pub async fn perform_self_test(&mut self) -> Result<bool, &'static str> {
        let mut buf = [0u8; 3];

        self.send_command(&CMD_PERFORM_SELF_TEST).await?;
        Timer::after_millis(PERFORM_SELF_TEST_DELAY.into()).await;

        match self.read_sequence(&CMD_PERFORM_SELF_TEST, &mut buf).await {
            Ok(()) => {
                let result = u16::from_be_bytes([buf[0], buf[1]]);
                if result == 0 {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(e) => Err(e),
        }
    }

    pub async fn start_periodic_measurement(&mut self) -> Result<(), &'static str> {
        self.send_command(&CMD_START_PERIODIC_MEASUREMENT)
            .await
            .map_err(|_| "Failed to start measurement")?;
        Timer::after_millis(INITIAL_MEASURE_DELAY.into()).await;
        Ok(())
    }

    pub async fn get_data_ready_status(&mut self) -> Result<bool, &'static str> {
        let mut buf = [0u8; 3];
        match self
            .read_sequence(&CMD_GET_DATA_READY_STATUS, &mut buf)
            .await
        {
            Ok(()) => {
                if ((buf[0] & 0x07) == 0) && (buf[1] == 0) {
                    Ok(false)
                } else {
                    Ok(true)
                }
            }
            Err(e) => Err(e),
        }
    }

    pub async fn read_measurement(&mut self) -> Result<(u16, f32, f32), &'static str> {
        let mut buf = [0u8; 9];
        if let Ok(true) = self.get_data_ready_status().await {
            match self.read_sequence(&CMD_READ_MEASUREMENT, &mut buf).await {
                Ok(()) => {
                    let co2 = u16::from_be_bytes([buf[0], buf[1]]);
                    let temperature =
                        -45.0 + 175.0 * (u16::from_be_bytes([buf[3], buf[4]]) as f32 / 65535.0);
                    let humidity = 100.0 * (u16::from_be_bytes([buf[6], buf[7]]) as f32 / 65535.0);

                    Ok((co2, temperature, humidity))
                }
                Err(e) => Err(e),
            }
        } else {
            Err("Sensor not ready yet. Try again in a few seconds")
        }
    }

    pub async fn stop_periodic_measurement(&mut self) -> Result<(), &'static str> {
        self.send_command(&CMD_STOP_PERIODIC_MEASUREMENT)
            .await
            .map_err(|_| "Failed to stop measurement")?;
        Timer::after_millis(STOP_MEASURE_DELAY.into()).await;
        Ok(())
    }

    pub async fn get_temp_offset(&mut self) -> Result<f32, &'static str> {
        let mut buf = [0u8; 3];
        match self
            .read_sequence(&CMD_GET_TEMPERATURE_OFFSET, &mut buf)
            .await
        {
            Ok(()) => {
                let raw_offset = u16::from_be_bytes([buf[0], buf[1]]);
                let temp_offset = raw_offset as f32 * 175.0 / 65535.0;
                Ok(temp_offset)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn get_sensor_altitude(&mut self) -> Result<u16, &'static str> {
        let mut buf = [0u8; 3];
        match self.read_sequence(&CMD_GET_SENSOR_ALTITUDE, &mut buf).await {
            Ok(()) => {
                let altitude = u16::from_be_bytes([buf[0], buf[1]]);
                Ok(altitude)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn get_ambient_pressure(&mut self) -> Result<u32, &'static str> {
        let mut buf = [0u8; 3];
        match self
            .read_sequence(&CMD_GET_AMBIENT_PRESSURE, &mut buf)
            .await
        {
            Ok(()) => {
                let raw_pressure = u16::from_be_bytes([buf[0], buf[1]]);
                let pressure_pa = u32::from(raw_pressure) * 100;
                Ok(pressure_pa)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn set_temperature_offset(
        &mut self,
        current_temp: f32,
        reference_temp: f32,
    ) -> Result<(), &'static str> {
        let previous_offset = match self.get_temp_offset().await {
            Ok(offset) => offset,
            Err(e) => return Err(e),
        };

        //given eqn in the datasheet
        let actual_offset = current_temp - reference_temp + previous_offset;

        // Check if the calculated offset is within the recommended range, this is not a hard limit though
        if actual_offset < 0.0 || actual_offset > 20.0 {
            return Err("Calculated temperature offset must be between 0 °C and 20 °C");
        }

        // word[0] = Toffset[°C] * (2^16 - 1) / 175
        let raw_offset = ((actual_offset * 65535.0) / 175.0) as u16;
        let offset_bytes = raw_offset.to_be_bytes();

        self.write_command(&CMD_SET_TEMPERATURE_OFFSET, &offset_bytes)
            .await
    }

    //this value must be in meters
    pub async fn set_sensor_altitude(&mut self, altitude: u16) -> Result<(), &'static str> {
        if altitude > 3000 {
            return Err("Altitude must be between 0 and 3000 meters");
        }

        let altitude_bytes = altitude.to_be_bytes();

        self.write_command(&CMD_SET_SENSOR_ALTITUDE, &altitude_bytes)
            .await
    }

    pub async fn set_ambient_pressure(&mut self, pressure: u32) -> Result<(), &'static str> {
        if pressure < 70_000 || pressure > 120_000 {
            return Err("Pressure must be between 70,000 and 120,000 Pa");
        }

        let pressure_raw = (pressure / 100) as u16; // Convert Pa to 100 Pa units
        let pressure_bytes = pressure_raw.to_be_bytes();

        self.write_command(&CMD_SET_AMBIENT_PRESSURE, &pressure_bytes)
            .await
    }

    //reserved for later use in case we decide to use perform_forced_recalibration
    // async fn send_command_and_fetch_result(
    //     &mut self,
    //     address: &[u8],
    //     data: &[u8],
    //     result: &mut [u8],
    // ) -> Result<(), &'static str> {
    //     // Send command using the existing write_command function
    //     self.write_command(address, data).await?;

    //     self.i2c
    //         .read(self.i2c_address, result)
    //         .await
    //         .map_err(|_| "Error fetching result")?;

    //     let result_data_len = result.len() - 1;
    //     let received_crc = result[result_data_len];
    //     let calculated_crc = self.crc8(&result[..result_data_len]);
    //     if received_crc != calculated_crc {
    //         return Err("CRC mismatch in result");
    //     }

    //     Ok(())
    // }
}
