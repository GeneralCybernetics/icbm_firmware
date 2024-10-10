use core::num;

use defmt::info;
use embassy_stm32::mode::Async;
use embassy_stm32::usart::{Error, Uart};
use heapless::String;
use itoa::Buffer;

const CO2_SCALE_VALUE: i32 = 100;
pub enum Mode {
    Sleep,
    Streaming,
    Polling,
}

//these values can be expanded significantly later (ex: LED Signal) but are unnecessay at the moment, so I have decided to only include these 3:
pub enum OutputValues {
    FilteredCO2,
    UnfilteredCO2,
    Both,
}

pub struct ExplorIrME100<'d> {
    uart: Uart<'d, Async>,
    mode: Mode,
    output_value: OutputValues,
}

impl<'d> ExplorIrME100<'d> {
    pub fn new(uart: Uart<'d, Async>) -> Self {
        let mut uninit = ExplorIrME100 {
            uart: uart,
            mode: Mode::Streaming,
            output_value: OutputValues::FilteredCO2,
        };

        uninit.change_mode(Mode::Streaming);
        uninit.change_output(OutputValues::FilteredCO2);

        let init = uninit;
        init
    }

    pub async fn change_mode(&mut self, mode: Mode) -> Result<(), Error> {
        let cmd = match mode {
            Mode::Sleep => b"K 0\r\n",
            Mode::Streaming => b"K 1\r\n",
            Mode::Polling => b"K 2\r\n",
        };

        match self.uart.write(cmd).await {
            Ok(_) => {
                self.mode = mode;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub async fn change_output(&mut self, output_value: OutputValues) -> Result<(), Error> {
        let cmd = match output_value {
            OutputValues::UnfilteredCO2 => b"M 2\r\n",
            OutputValues::FilteredCO2 => b"M 4\r\n",
            OutputValues::Both => b"M 6\r\n",
        };

        match self.uart.write(cmd).await {
            Ok(_) => {
                self.output_value = output_value;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub async fn get_filtered_co2(&mut self) -> Result<i32, &'static str> {
        let cmd = b"Z\r\n";

        match self.uart.write(cmd).await {
            Ok(_) => {}
            Err(_) => return Err("Failed to write to UART"),
        }

        let mut response = [0u8; 10];

        match self.uart.read(&mut response).await {
            Ok(_) => {}
            Err(_) => return Err("Failed to read from UART"),
        }

        let co2_reading = match core::str::from_utf8(&response[3..=7]) {
            Ok(s) => s,
            Err(_) => return Err("Failed to parse response as UTF-8"),
        };

        match co2_reading.parse::<i32>() {
            Ok(num) => Ok(num * CO2_SCALE_VALUE),
            Err(_) => Err("Failed to parse CO2 reading as integer"),
        }
    }

    pub async fn get_unfiltered_co2(&mut self) -> Result<i32, &'static str> {
        let cmd = b"z\r\n";

        match self.uart.write(cmd).await {
            Ok(_) => {}
            Err(_) => return Err("Failed to write to UART"),
        }

        let mut response = [0u8; 10];

        match self.uart.read(&mut response).await {
            Ok(_) => {}
            Err(_) => return Err("Failed to read from UART"),
        }

        let co2_reading = match core::str::from_utf8(&response[3..=7]) {
            Ok(s) => s,
            Err(_) => return Err("Failed to parse response as UTF-8"),
        };

        match co2_reading.parse::<i32>() {
            Ok(num) => Ok(num * CO2_SCALE_VALUE),
            Err(_) => Err("Failed to parse CO2 reading as integer"),
        }
    }

    pub async fn get_pressure_and_concentration(&mut self) -> Result<i32, &'static str> {
        let cmd = b"s\r\n";

        match self.uart.write(cmd).await {
            Ok(_) => {}
            Err(_) => return Err("Failed to write to UART"),
        }

        let mut response = [0u8; 10];

        match self.uart.read(&mut response).await {
            Ok(_) => {}
            Err(_) => return Err("Failed to read from UART"),
        }

        info!("{:#x}", &response);

        let co2_reading = match core::str::from_utf8(&response[3..=7]) {
            Ok(s) => s,
            Err(_) => return Err("Failed to parse response as UTF-8"),
        };

        match co2_reading.parse::<i32>() {
            Ok(num) => Ok(num),
            Err(_) => Err("Failed to parse Pressure & Concentration reading as integer"),
        }
    }

    //altitude must be in meter
    pub async fn set_pressure_and_concentration(
        &mut self,
        altitude: f32,
    ) -> Result<(), &'static str> {
        if altitude < 0.0 || altitude > 3050.0 {
            return Err("altitude out of range");
        }

        //I charted the values and this quadratic curve fit the comp val best
        let compensation_value = ((-0.000043713 * altitude as f64 * altitude as f64)
            + (1.2813 * altitude as f64)
            + 8229.2) as i32;

        // info!("{:?}", compensation_value);
        let mut buffer = itoa::Buffer::new();
        let num_str = buffer.format(compensation_value);

        let mut cmd = [0u8; 10];
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

    pub async fn calibrate(&mut self, ppm: u32) -> Result<(), &'static str> {
        let scaled_val = ppm / 100;
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

    pub async fn fine_tune(&mut self, ppm: u32, sensor_output: u32) -> Result<(), &'static str> {
        let scaled_ppm = ppm;
        let scaled_output = sensor_output;

        let mut buffer_ppm = itoa::Buffer::new();
        let ppm_str = buffer_ppm.format(scaled_ppm);

        let mut buffer_output = itoa::Buffer::new();
        let output_str = buffer_output.format(scaled_output);

        let mut cmd = [0u8; 20];
        let mut index = 0;

        // Start with "F "
        index += b"F ".len();
        cmd[0..index].copy_from_slice(b"F ");

        // Add the PPM value
        cmd[index..index + ppm_str.len()].copy_from_slice(ppm_str.as_bytes());
        index += ppm_str.len();

        // Add a space between the PPM and output value
        cmd[index] = b' ';
        index += 1;

        // Add the sensor output value
        cmd[index..index + output_str.len()].copy_from_slice(output_str.as_bytes());
        index += output_str.len();

        // Add carriage return and newline (\r\n)
        cmd[index..index + 2].copy_from_slice(b"\r\n");

        // Send the command over UART
        self.uart
            .write(&cmd)
            .await
            .map_err(|_| "Failed to write F fine-tune command to UART")?;

        Ok(())
    }

    pub async fn read_serial_no(&mut self) -> Result<String<47>, &'static str> {
        let cmd = b"Y\r\n";
        self.uart
            .write(cmd)
            .await
            .map_err(|_| "Failed to write command")?;

        let mut buf = [0u8; 47]; // Buffer to hold the response, size 47

        self.uart
            .read(&mut buf)
            .await
            .map_err(|_| "Failed to read response")?;

        self.parse_response(&buf[..], 'Y')
    }

    fn parse_response(&self, resp: &[u8], check_letter: char) -> Result<String<47>, &'static str> {
        const ASCII_SPACE: u8 = 0x20;
        const CR: u8 = 0x0D;
        const LF: u8 = 0x0A;
        let char_ascii = check_letter as u8;
        let arr_len = resp.len();

        if arr_len < 4 {
            return Err("Response too short");
        }

        if resp[0] == ASCII_SPACE
            && resp[arr_len - 2] == CR
            && resp[arr_len - 1] == LF
            && resp[1] == char_ascii
        {
            // Use heapless String to store the response
            let mut result: String<47> = String::new();
            if let Ok(parsed_str) = core::str::from_utf8(&resp[..arr_len - 2]) {
                result.push_str(parsed_str).map_err(|_| "String overflow")?;
                Ok(result)
            } else {
                Err("Invalid UTF-8 sequence")
            }
        } else {
            Err("Error parsing response")
        }
    }
}
