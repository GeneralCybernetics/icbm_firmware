use defmt::{error, info};
use embassy_stm32::mode::Async;
use embassy_stm32::usart::{Error, Uart};
use embassy_time::Timer;
use heapless::String;
use libm::pow;

// Timing Constants
const TIME_TO_FIRST_VAL: u64 = 1200; //ms

// Scaling Constants
const CO2_SCALE_VALUE: i32 = 100;

// UART Commands
const CMD_SLEEP: &[u8] = b"K 0\r\n";
const CMD_STREAMING: &[u8] = b"K 1\r\n";
const CMD_POLLING: &[u8] = b"K 2\r\n";
const CMD_GET_FILTERED_CO2: &[u8] = b"Z\r\n";
const CMD_GET_UNFILTERED_CO2: &[u8] = b"z\r\n";
const CMD_GET_PRESSURE_COMP: &[u8] = b"s\r\n";
// Development-only command constant - remove the underscore prefix
// when implementing digital filter status checks in a future update
const _CMD_GET_DIGITAL_FILTER: &[u8] = b"a\r\n";
const CMD_SET_DIGITAL_FILTER_32: &[u8] = b"A 32\r\n";
const CMD_GET_SERIAL: &[u8] = b"Y\r\n";

// Response Buffer Sizes
const RESPONSE_BUFFER_SIZE: usize = 10;
const SERIAL_BUFFER_SIZE: usize = 47;

// Pressure Compensation Limits
const MIN_PRESSURE_MBAR: f32 = 300.0;
const MAX_PRESSURE_MBAR: f32 = 1100.0;
const SEA_LEVEL_PRESSURE: f32 = 1013.0;

//verified
#[derive(Debug)]
pub enum ResponseError {
    TooShort,
    InvalidFormat,
    MissingSpace,
    InvalidTermination,
    WrongCommand,
    Utf8Error,
    StringOverflow,
}

pub enum Mode {
    Sleep,
    Streaming,
    Polling,
}

pub struct ExplorIrME100<'d> {
    uart: Uart<'d, Async>,
    mode: Mode,
}

impl<'d> ExplorIrME100<'d> {
    pub fn new(uart: Uart<'d, Async>) -> Self {
        ExplorIrME100 {
            uart: uart,
            mode: Mode::Polling,
        }
    }

    pub async fn init(&mut self) -> Result<(), &'static str> {
        self.change_mode(Mode::Polling).await.map_err(|e| {
            error!("Failed to set polling mode: {}", e);
            "Failed to initialize sensor in polling mode"
        })?;
        info!("Successfully set sensor to polling mode");

        self.uart
            .write(CMD_SET_DIGITAL_FILTER_32)
            .await
            .map_err(|_| "Failed to write digital filter command to UART")?;
        info!("Set Digital Filter command sent");

        Timer::after_millis(TIME_TO_FIRST_VAL).await;

        info!("Sensor initialization completed successfully");
        Ok(())
    }

    pub async fn change_mode(&mut self, mode: Mode) -> Result<(), Error> {
        let cmd = match mode {
            Mode::Sleep => CMD_SLEEP,
            Mode::Streaming => CMD_STREAMING,
            Mode::Polling => CMD_POLLING,
        };

        match self.uart.write(cmd).await {
            Ok(_) => {
                self.mode = mode;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    //returns the value in ppm as i32
    pub async fn get_filtered_co2(&mut self) -> Result<i32, &'static str> {
        self.uart
            .write(CMD_GET_FILTERED_CO2)
            .await
            .map_err(|_| "Failed to write to UART")?;

        let mut response = [0u8; RESPONSE_BUFFER_SIZE];
        self.uart
            .read(&mut response)
            .await
            .map_err(|_| "Failed to read from UART")?;

        let result = self
            .parse_response::<10>(&response, 'Z')
            .map_err(Self::map_response_error)?;

        let value = result
            .trim()
            .parse::<i32>()
            .map_err(|_| "Failed to parse CO2 reading as integer")?;

        Ok(value * CO2_SCALE_VALUE)
    }

    //returns the value in ppm as i32
    pub async fn get_unfiltered_co2(&mut self) -> Result<i32, &'static str> {
        self.uart
            .write(CMD_GET_UNFILTERED_CO2)
            .await
            .map_err(|_| "Failed to write to UART")?;

        let mut response = [0u8; RESPONSE_BUFFER_SIZE];
        self.uart
            .read(&mut response)
            .await
            .map_err(|_| "Failed to read from UART")?;

        let result = self
            .parse_response::<10>(&response, 'z')
            .map_err(Self::map_response_error)?;

        let value = result
            .trim()
            .parse::<i32>()
            .map_err(|_| "Failed to parse CO2 reading as integer")?;

        Ok(value * CO2_SCALE_VALUE)
    }

    //reports in compensation value
    pub async fn get_pressure_and_concentration(&mut self) -> Result<i32, &'static str> {
        self.uart
            .write(CMD_GET_PRESSURE_COMP)
            .await
            .map_err(|_| "Failed to write to UART")?;

        let mut response = [0u8; RESPONSE_BUFFER_SIZE];
        self.uart
            .read(&mut response)
            .await
            .map_err(|_| "Failed to read from UART")?;

        let result = self
            .parse_response::<10>(&response, 's')
            .map_err(Self::map_response_error)?;

        result
            .trim()
            .parse::<i32>()
            .map_err(|_| "Failed to parse pressure reading as integer")
    }

    //Input: millibars (mBar), range 300-1100
    pub async fn set_pressure_and_concentration(
        &mut self,
        pressure_mbar: f32,
    ) -> Result<(), &'static str> {
        if pressure_mbar < MIN_PRESSURE_MBAR || pressure_mbar > MAX_PRESSURE_MBAR {
            return Err("pressure out of range (300-1100 mbar)");
        }

        let sea_level_difference = pressure_mbar - SEA_LEVEL_PRESSURE;
        let compensation_value = (8192.0 + (sea_level_difference * 0.14 / 100.0) * 8192.0) as i32;

        let mut buffer = itoa::Buffer::new();
        let num_str = buffer.format(compensation_value);

        // Construct the command string "S <value>\r\n"
        let mut cmd = [0u8; 9];
        let mut index = 0;
        index += b"S ".len();
        cmd[0..index].copy_from_slice(b"S ");
        cmd[index..index + num_str.len()].copy_from_slice(num_str.as_bytes());
        index += num_str.len();
        cmd[index..index + 2].copy_from_slice(b"\r\n");

        self.uart
            .write(&cmd)
            .await
            .map_err(|_| "Failed to write to UART")
    }

    //input the value in ppm
    pub async fn calibrate(&mut self, ppm: u32) -> Result<(), &'static str> {
        let scaled_val = ppm / CO2_SCALE_VALUE as u32;
        let mut buffer = itoa::Buffer::new();
        let num_str = buffer.format(scaled_val);

        let mut cmd = [0u8; 10];
        let mut index = 0;
        index += b"X ".len();
        cmd[0..index].copy_from_slice(b"X ");
        cmd[index..index + num_str.len()].copy_from_slice(num_str.as_bytes());
        index += num_str.len();
        cmd[index..index + 2].copy_from_slice(b"\r\n");

        self.uart
            .write(&cmd)
            .await
            .map_err(|_| "Failed to write X command to UART")?;
        Ok(())
    }

    //input both the values in ppm
    pub async fn fine_tune(&mut self, ppm: u32, sensor_output: u32) -> Result<(), &'static str> {
        let scaled_ppm = ppm / CO2_SCALE_VALUE as u32;
        let scaled_output = sensor_output / CO2_SCALE_VALUE as u32;

        let mut buffer_ppm = itoa::Buffer::new();
        let ppm_str = buffer_ppm.format(scaled_ppm);

        let mut buffer_output = itoa::Buffer::new();
        let output_str = buffer_output.format(scaled_output);

        let mut cmd = [0u8; 20];
        let mut index = 0;

        index += b"F ".len();
        cmd[0..index].copy_from_slice(b"F ");

        cmd[index..index + output_str.len()].copy_from_slice(output_str.as_bytes());
        index += output_str.len();

        cmd[index] = b' ';
        index += 1;

        cmd[index..index + ppm_str.len()].copy_from_slice(ppm_str.as_bytes());
        index += ppm_str.len();

        cmd[index..index + 2].copy_from_slice(b"\r\n");

        self.uart
            .write(&cmd)
            .await
            .map_err(|_| "Failed to write F fine-tune command to UART")?;

        Ok(())
    }

    pub async fn read_serial_no(&mut self) -> Result<String<SERIAL_BUFFER_SIZE>, &'static str> {
        self.uart
            .write(CMD_GET_SERIAL)
            .await
            .map_err(|_| "Failed to write command")?;

        let mut response = [0u8; SERIAL_BUFFER_SIZE];
        self.uart
            .read(&mut response)
            .await
            .map_err(|_| "Failed to read from UART")?;

        self.parse_response::<SERIAL_BUFFER_SIZE>(&response, 'Y')
            .map_err(Self::map_response_error)
    }

    fn parse_response<const N: usize>(
        &self,
        resp: &[u8],
        check_letter: char,
    ) -> Result<String<N>, ResponseError> {
        const ASCII_SPACE: u8 = 0x20;
        const CR: u8 = 0x0D;
        const LF: u8 = 0x0A;

        if resp.len() < 4 {
            return Err(ResponseError::TooShort);
        }

        if resp[0] != ASCII_SPACE {
            return Err(ResponseError::MissingSpace);
        }

        if resp[1] != check_letter as u8 {
            return Err(ResponseError::WrongCommand);
        }

        let end_idx = resp.len() - 2;
        if resp[end_idx] != CR || resp[end_idx + 1] != LF {
            return Err(ResponseError::InvalidTermination);
        }

        let mut result = String::<N>::new();
        match core::str::from_utf8(&resp[2..end_idx]) {
            Ok(parsed_str) => match result.push_str(parsed_str) {
                Ok(_) => Ok(result),
                Err(_) => Err(ResponseError::StringOverflow),
            },
            Err(_) => Err(ResponseError::Utf8Error),
        }
    }

    fn map_response_error(e: ResponseError) -> &'static str {
        match e {
            ResponseError::TooShort => "Response too short",
            ResponseError::MissingSpace => "Missing leading space",
            ResponseError::WrongCommand => "Wrong command letter in response",
            ResponseError::InvalidTermination => "Invalid termination sequence",
            ResponseError::Utf8Error => "Invalid UTF-8 in response",
            ResponseError::StringOverflow => "Response string overflow",
            ResponseError::InvalidFormat => "Invalid response format",
        }
    }

    // Verified function; currently unused - remove #[allow(dead_code)]
    // when this is integrated into the CO2 compensation calculations
    #[allow(dead_code)]
    fn calculate_y(c1: f64) -> f64 {
        2.811e-38 * pow(c1, 6.0) - 9.817e-32 * pow(c1, 5.0) + 1.304e-25 * pow(c1, 4.0)
            - 8.126e-20 * pow(c1, 3.0)
            + 2.311e-14 * pow(c1, 2.0)
            - 2.195e-9 * c1
            - 1.471e-3
    }
}
