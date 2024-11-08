use defmt::{error, info};
use embassy_stm32::i2c::{Error, I2c};
use embassy_stm32::mode::Async;
use embassy_time::Timer;

// I2C Address
const SCD41_I2C_ADDRESS: u8 = 0x62;

// CRC
const CRC8_INIT: u8 = 0xFF;
const CRC_POLYNOMIAL: u8 = 0x31;

// Basic Commands
const CMD_START_PERIODIC_MEASUREMENT: [u8; 2] = [0x21, 0xB1];
const CMD_READ_MEASUREMENT: [u8; 2] = [0xEC, 0x05];
const CMD_STOP_PERIODIC_MEASUREMENT: [u8; 2] = [0x3F, 0x86];

// On-Chip Output Signal Compensation
const CMD_SET_TEMPERATURE_OFFSET: [u8; 2] = [0x24, 0x1D];
const CMD_GET_TEMPERATURE_OFFSET: [u8; 2] = [0x23, 0x18];
const CMD_SET_SENSOR_ALTITUDE: [u8; 2] = [0x24, 0x27];
const CMD_GET_SENSOR_ALTITUDE: [u8; 2] = [0x23, 0x22];
const CMD_SET_AMBIENT_PRESSURE: [u8; 2] = [0xE0, 0x00];
const CMD_GET_AMBIENT_PRESSURE: [u8; 2] = [0xE0, 0x00];

// Field Calibration
const CMD_SET_AUTOMATIC_SELF_CALIBRATION_ENABLED: [u8; 2] = [0x24, 0x16];

// Low Power Periodic Measurement Mode
const CMD_GET_DATA_READY_STATUS: [u8; 2] = [0xE4, 0xB8];

// Advanced Features
const CMD_GET_SERIAL_NUMBER: [u8; 2] = [0x36, 0x82];
const CMD_PERFORM_SELF_TEST: [u8; 2] = [0x36, 0x39];

// Execution times (in milliseconds)
const POWERUP_TIME: u64 = 30;
const INITIAL_MEASURE_DELAY: u64 = 500;
const STOP_MEASURE_DELAY: u64 = 500;
const EXECUTION_TIME_PERFORM_SELF_TEST: u64 = 10_000;
const EXECUTION_TIME_READ_MEASUREMENT: u64 = 1;
const EXECUTION_TIME_GET_TEMPERATURE_OFFSET: u64 = 1;
const EXECUTION_TIME_GET_SENSOR_ALTITUDE: u64 = 1;
const EXECUTION_TIME_GET_AMBIENT_PRESSURE: u64 = 1;
const EXECUTION_TIME_GET_DATA_READY_STATUS: u64 = 1;
const DATA_READY_LOOP_DELAY: u64 = 3000;
const EXECUTION_TIME_GET_SERIAL_NUMBER: u64 = 1;

//Attempts counts
const DATA_READ_MAX_ATTEMPTS: u8 = 5;

enum SCD41State {
    Idle,
    Measurement,
}

pub enum SensorSettings {
    Custom {
        current_temp: f32,
        reference_temp: f32,
        pressure: u32,
        altitude: u16,
    },
    Default,
}

pub struct SCD41<'d> {
    i2c: I2c<'d, Async>,
    i2c_address: u8,
    scd41_state: SCD41State,
}
impl<'d> SCD41<'d> {
    //N.B. Initialize the sensor using `init()` before any operations
    pub fn new(i2c: I2c<'d, Async>) -> Self {
        SCD41 {
            i2c,
            i2c_address: SCD41_I2C_ADDRESS,
            scd41_state: SCD41State::Measurement, //Default state is set to Measurement as per observed behavior, though datasheet indicates Idle as initial state
        }
    }

    pub async fn init(&mut self, settings: Option<SensorSettings>) -> Result<(), &'static str> {
        Timer::after_millis(POWERUP_TIME).await;

        if let Err(e) = self.stop_periodic_measurement().await {
            error!("Failed to stop periodic measurement: {}", e);
            return Err("Failed to stop periodic measurement");
        }
        info!("CMD_STOP_PERIODIC_MEASUREMENT successfully sent");

        match self.perform_self_test().await {
            Ok(true) => {
                info!("SCD41 self-test passed");
            }
            Ok(false) => {
                error!("Sensor malfunction detected -- this could be either physical or temporary");
                return Err("Sensor malfunction detected");
            }
            Err(e) => {
                error!("Error while trying to perform self test: {}", e);
                return Err("Failed to perform self-test");
            }
        }

        // Note: Despite the function name containing "enabled", this actually disables calibration.
        // Function name matches datasheet for consistency, but sends disable command [0x00, 0x00]
        if let Err(e) = self.set_automatic_self_calibration_enabled().await {
            error!("Failed to disable automatic self calibration: {}", e);
            return Err("Failed to disable automatic self calibration");
        }
        info!("Automatic self calibration disabled successfully");

        match settings {
            Some(SensorSettings::Custom {
                current_temp,
                reference_temp,
                pressure,
                altitude,
            }) => {
                match self
                    .set_internals(current_temp, reference_temp, pressure, altitude)
                    .await
                {
                    Ok(_) => info!("Internal sensor settings successfully applied"),
                    Err(e) => error!("Failed to set internal sensor settings: {}", e),
                }
            }
            Some(SensorSettings::Default) | None => {
                let current_offset = match self.get_temp_offset().await {
                    Ok(offset) => offset,
                    Err(e) => {
                        error!("Failed to get current temperature offset: {}", e);
                        return Err("Failed to initialize sensor");
                    }
                };

                match self.set_internals(4.0, current_offset, 101_300, 0).await {
                    Ok(_) => info!("Default sensor settings successfully applied"),
                    Err(e) => error!("Failed to set default sensor settings: {}", e),
                }
            }
        }

        if let Err(e) = self.start_periodic_measurement().await {
            error!("Failed to start periodic measurement: {}", e);
            return Err("Failed to start periodic measurement");
        }
        info!("CMD_START_PERIODIC_MEASUREMENT successfully sent");

        Ok(())
    }

    pub async fn start_periodic_measurement(&mut self) -> Result<(), &'static str> {
        if let Err(e) = self.send_command(&CMD_START_PERIODIC_MEASUREMENT).await {
            error!("Failed to start periodic measurement: {}", e);
            return Err("Failed to start periodic measurement");
        }
        self.scd41_state = SCD41State::Measurement;
        Timer::after_millis(INITIAL_MEASURE_DELAY).await;
        Ok(())
    }

    pub async fn read_measurement(&mut self) -> Result<(u16, f32, f32), &'static str> {
        let mut buf = [0u8; 9];
        let mut attempts = 0;

        while attempts < DATA_READ_MAX_ATTEMPTS {
            if let Ok(true) = self.get_data_ready_status().await {
                info!("Data ready; reading sensor");
                match self
                    .read_sequence(
                        &CMD_READ_MEASUREMENT,
                        &mut buf,
                        EXECUTION_TIME_READ_MEASUREMENT,
                    )
                    .await
                {
                    Ok(()) => {
                        let co2 = u16::from_be_bytes([buf[0], buf[1]]);
                        let temperature =
                            -45.0 + 175.0 * (u16::from_be_bytes([buf[3], buf[4]]) as f32 / 65535.0);
                        let humidity =
                            100.0 * (u16::from_be_bytes([buf[6], buf[7]]) as f32 / 65535.0);
                        return Ok((co2, temperature, humidity));
                    }
                    Err(e) => return Err(e),
                }
            } else {
                if attempts < DATA_READ_MAX_ATTEMPTS - 1 {
                    info!("Data not ready; retrying in {}ms", DATA_READY_LOOP_DELAY);
                    Timer::after_millis(DATA_READY_LOOP_DELAY).await;
                    attempts += 1;
                } else {
                    return Err("Data not ready after max attempts");
                }
            }
        }

        Err("Unexpected error in read_measurement")
    }

    pub async fn stop_periodic_measurement(&mut self) -> Result<(), &'static str> {
        if let Err(e) = self.send_command(&CMD_STOP_PERIODIC_MEASUREMENT).await {
            error!("Failed to stop periodic measurement: {}", e);
            return Err("Failed to stop periodic measurement");
        }
        self.scd41_state = SCD41State::Idle;
        Timer::after_millis(STOP_MEASURE_DELAY).await;
        Ok(())
    }

    pub async fn get_temp_offset(&mut self) -> Result<f32, &'static str> {
        self.ensure_idle().await?;
        let mut buf = [0u8; 3];
        match self
            .read_sequence(
                &CMD_GET_TEMPERATURE_OFFSET,
                &mut buf,
                EXECUTION_TIME_GET_TEMPERATURE_OFFSET,
            )
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
        self.ensure_idle().await?;
        let mut buf = [0u8; 3];
        match self
            .read_sequence(
                &CMD_GET_SENSOR_ALTITUDE,
                &mut buf,
                EXECUTION_TIME_GET_SENSOR_ALTITUDE,
            )
            .await
        {
            Ok(()) => {
                // info!("{:#x}", buf);
                let altitude = u16::from_be_bytes([buf[0], buf[1]]);
                Ok(altitude)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn get_ambient_pressure(&mut self) -> Result<u32, &'static str> {
        let mut buf = [0u8; 3];
        match self
            .read_sequence(
                &CMD_GET_AMBIENT_PRESSURE,
                &mut buf,
                EXECUTION_TIME_GET_AMBIENT_PRESSURE,
            )
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

    // Recommended range: 0-20°C
    pub async fn set_temp_offset(
        &mut self,
        current_temp: f32,
        reference_temp: f32,
    ) -> Result<(), &'static str> {
        self.ensure_idle().await?;
        let previous_offset = match self.get_temp_offset().await {
            Ok(offset) => offset,
            Err(e) => return Err(e),
        };

        // Calculate temperature offset: current_temp - reference_temp + previous_offset
        let actual_offset = current_temp - reference_temp + previous_offset;

        if actual_offset < 0.0 || actual_offset > 20.0 {
            error!("Calculated temperature offset outside of recommended range: 0 °C and 20 °C");
        }

        // word[0] = Toffset[°C] * (2^16 - 1) / 175
        let raw_offset = ((actual_offset * 65535.0) / 175.0) as u16;
        let offset_bytes = raw_offset.to_be_bytes();

        self.write_command(&CMD_SET_TEMPERATURE_OFFSET, &offset_bytes)
            .await
    }

    // Altitude range: 0-3000 meters above sea level
    pub async fn set_sensor_altitude(&mut self, altitude: u16) -> Result<(), &'static str> {
        self.ensure_idle().await?;
        if altitude > 3000 {
            return Err("Altitude must be between 0 and 3000 meters");
        }

        let altitude_bytes = altitude.to_be_bytes();

        self.write_command(&CMD_SET_SENSOR_ALTITUDE, &altitude_bytes)
            .await
    }

    // Pressure range: 70-120 kPa
    pub async fn set_ambient_pressure(&mut self, pressure: u32) -> Result<(), &'static str> {
        if pressure < 70_000 || pressure > 120_000 {
            return Err("Pressure must be between 70,000 and 120,000 Pa");
        }

        let pressure_raw = (pressure / 100) as u16; // Convert Pa to 100 Pa units
        let pressure_bytes = pressure_raw.to_be_bytes();

        self.write_command(&CMD_SET_AMBIENT_PRESSURE, &pressure_bytes)
            .await
    }

    //Returns 48-bit serial number with CRC bytes, ex: [0x7d, 0x6b, 0xab, 0x7b, 0x7, 0x37, 0x3b, 0x12, 0x8]
    pub async fn get_serial_number(&mut self) -> Result<[u8; 9], &'static str> {
        self.ensure_idle().await?;
        let mut buf = [0u8; 9]; // 3 words, each followed by CRC (3 * (2 + 1) = 9)

        match self
            .read_sequence(
                &CMD_GET_SERIAL_NUMBER,
                &mut buf,
                EXECUTION_TIME_GET_SERIAL_NUMBER,
            )
            .await
        {
            Ok(()) => Ok(buf),
            Err(e) => Err(e),
        }
    }

    async fn set_internals(
        &mut self,
        current_temp: f32,
        reference_temp: f32,
        pressure: u32,
        altitude: u16,
    ) -> Result<(), &'static str> {
        self.ensure_idle().await?;

        self.set_ambient_pressure(pressure).await?;
        match self.get_ambient_pressure().await {
            Ok(set_pressure) => info!("Ambient pressure has been set to: {} Pa", set_pressure),
            Err(e) => error!("Failed to get ambient pressure: {}", e),
        }

        self.set_sensor_altitude(altitude).await?;
        match self.get_sensor_altitude().await {
            Ok(set_altitude) => info!("Sensor altitude has been set to: {} m", set_altitude),
            Err(e) => error!("Failed to get sensor altitude: {}", e),
        }

        self.set_temp_offset(current_temp, reference_temp).await?;
        match self.get_temp_offset().await {
            Ok(offset) => info!("Temperature offset has been set to: {} °C", offset),
            Err(e) => error!("Failed to get temperature offset: {}", e),
        }

        Ok(())
    }

    async fn perform_self_test(&mut self) -> Result<bool, &'static str> {
        self.ensure_idle().await?;
        let mut buf = [0u8; 3];

        match self
            .read_sequence(
                &CMD_PERFORM_SELF_TEST,
                &mut buf,
                EXECUTION_TIME_PERFORM_SELF_TEST,
            )
            .await
        {
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

    // Note: Despite the function name containing "enabled", this actually disables calibration.
    // Function name matches datasheet for consistency, but sends disable command [0x00, 0x00]
    async fn set_automatic_self_calibration_enabled(&mut self) -> Result<(), &'static str> {
        self.ensure_idle().await?;
        let disable_calibration = [0x00, 0x00];
        self.write_command(
            &CMD_SET_AUTOMATIC_SELF_CALIBRATION_ENABLED,
            &disable_calibration,
        )
        .await
    }

    async fn get_data_ready_status(&mut self) -> Result<bool, &'static str> {
        // Data ready interval: ~3000ms (Empirical)
        let mut buf = [0u8; 3];

        match self
            .read_sequence(
                &CMD_GET_DATA_READY_STATUS,
                &mut buf,
                EXECUTION_TIME_GET_DATA_READY_STATUS,
            )
            .await
        {
            Ok(()) => {
                // info!("{:#x}", buf);
                if ((buf[0] & 0x07) == 0) && (buf[1] == 0) {
                    Ok(false)
                } else {
                    Ok(true)
                }
            }
            Err(e) => Err(e),
        }
    }

    fn crc8(&self, data: &[u8]) -> u8 {
        let mut crc: u8 = CRC8_INIT;
        for &byte in data {
            crc ^= byte;
            for _ in 0..8 {
                if crc & 0x80 != 0 {
                    crc = (crc << 1) ^ CRC_POLYNOMIAL;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc
    }

    async fn send_command(&mut self, address: &[u8]) -> Result<(), &'static str> {
        self.i2c
            .write(self.i2c_address, address)
            .await
            .map_err(|e| match e {
                Error::Arbitration => "Error sending command: Arbitration",
                Error::Bus => "Error sending command: Bus",
                Error::Crc => "Error sending command: CRC",
                Error::Nack => "Error sending command: NACK",
                Error::Overrun => "Error sending command: Overrun",
                Error::Timeout => "Error sending command: Timeout",
                Error::ZeroLengthTransfer => "Error sending command: Zero Length Transfer",
            })
    }

    async fn write_command(&mut self, address: &[u8], data: &[u8]) -> Result<(), &'static str> {
        const MAX_LENGTH: usize = 5; // 2 (reg addr) + 2 (data) + 1(crc)
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
            .map_err(|e| match e {
                Error::Arbitration => "Error writing command: Arbitration",
                Error::Bus => "Error writing command: Bus",
                Error::Crc => "Error writing command: CRC",
                Error::Nack => "Error writing command: NACK",
                Error::Overrun => "Error writing command: Overrun",
                Error::Timeout => "Error writing command: Timeout",
                Error::ZeroLengthTransfer => "Error writing command: Zero Length Transfer",
            })
    }

    async fn read_sequence(
        &mut self,
        address: &[u8],
        buf: &mut [u8],
        millis: u64,
    ) -> Result<(), &'static str> {
        self.i2c
            .write(self.i2c_address, address)
            .await
            .map_err(|e| match e {
                Error::Arbitration => "Error writing while reading sequence: Arbitration",
                Error::Bus => "Error writing while reading sequence: Bus",
                Error::Crc => "Error writing while reading sequence: CRC",
                Error::Nack => "Error writing while reading sequence: NACK",
                Error::Overrun => "Error writing while reading sequence: Overrun",
                Error::Timeout => "Error writing while reading sequence: Timeout",
                Error::ZeroLengthTransfer => {
                    "Error writing while reading sequence: Zero Length Transfer"
                }
            })?;

        Timer::after_millis(millis).await;

        self.i2c
            .read(self.i2c_address, buf)
            .await
            .map_err(|e| match e {
                Error::Arbitration => "Error reading sequence: Arbitration",
                Error::Bus => "Error reading sequence: Bus",
                Error::Crc => "Error reading sequence: CRC",
                Error::Nack => "Error reading sequence: NACK",
                Error::Overrun => "Error reading sequence: Overrun",
                Error::Timeout => "Error reading sequence: Timeout",
                Error::ZeroLengthTransfer => "Error reading sequence: Zero Length Transfer",
            })?;

        let mut i = 0;

        while i < buf.len() {
            let remaining = buf.len() - i;
            if remaining >= 3 {
                let data = &buf[i..i + 2];
                let received_crc = buf[i + 2];
                let calculated_crc = self.crc8(data);
                if calculated_crc != received_crc {
                    error!("Received data: {:#x}", buf);
                    return Err("CRC mismatch in read data");
                }
                i += 3;
            } else {
                error!("Buffer size % 3 != 0; ignore if intended");
                break;
            }
        }

        Ok(())
    }

    async fn ensure_idle(&self) -> Result<(), &'static str> {
        match self.scd41_state {
            SCD41State::Idle => Ok(()),
            SCD41State::Measurement => Err("Sensor in measurement mode. Stop measurement first."),
        }
    }
}
