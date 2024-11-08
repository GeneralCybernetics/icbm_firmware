use defmt::info;
use embassy_stm32::adc::{Adc, AdcChannel, Instance, Resolution};
use embassy_stm32::gpio::Pin;
use libm::logf;

const THERMISTOR_R_DIVIDER: f32 = 10_000.0; // Voltage divider resistor (ohms)
const THERMISTOR_BETA: f32 = 3950.0; // Beta coefficient
const THERMISTOR_T0: f32 = 298.15; // Reference temperature (Kelvin)
const THERMISTOR_R0: f32 = 100_000.0; // Reference resistance at T0 (ohms)
const KELVIN_TO_CELSIUS: f32 = 273.15; // Conversion constant

pub struct Thermistor<'a, AdcReadPin: Pin + AdcChannel<AdcInstance>, AdcInstance: Instance> {
    adc_pin: AdcReadPin,
    adc: Adc<'a, AdcInstance>,
    resolution: Resolution,
}

impl<'a, AdcReadPin: Pin + AdcChannel<AdcInstance>, AdcInstance: Instance>
    Thermistor<'a, AdcReadPin, AdcInstance>
{
    pub fn new(adc_pin: AdcReadPin, mut adc: Adc<'a, AdcInstance>) -> Self {
        let resolution = Resolution::BITS12;
        adc.set_resolution(resolution);

        Thermistor {
            adc_pin,
            adc,
            resolution,
        }
    }

    fn get_thermistor_resistance(&mut self) -> f32 {
        let adc_value = self.adc.blocking_read(&mut self.adc_pin) as f32;
        info!("adc val {}", adc_value);

        let adc_max = match self.resolution {
            Resolution::BITS12 => 4095.0, // 2^12 - 1
            Resolution::BITS10 => 1023.0, // 2^10 - 1
            Resolution::BITS8 => 255.0,   // 2^8 - 1
            Resolution::BITS6 => 63.0,    // 2^6 - 1
        };

        THERMISTOR_R_DIVIDER * (adc_value / (adc_max - adc_value))
        // THERMISTOR_R_DIVIDER * (adc_max / adc_value - 1.0) //config 2
    }

    pub fn get_temperature_celsius(&mut self) -> f32 {
        let resistance = self.get_thermistor_resistance();

        // Steinhart-Hart equation (simplified beta equation)
        // 1/T = 1/T0 + (1/beta) * ln(R/R0)
        let temp_kelvin = 1.0
            / (1.0 / THERMISTOR_T0 + (1.0 / THERMISTOR_BETA) * logf(resistance / THERMISTOR_R0));

        // Convert Kelvin to Celsius
        temp_kelvin - KELVIN_TO_CELSIUS
    }
}
