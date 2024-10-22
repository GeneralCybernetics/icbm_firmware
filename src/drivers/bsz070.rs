use defmt::error;
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{Ch1, PwmPin, SimplePwm};
use embassy_stm32::timer::{Channel, GeneralInstance4Channel};
use embassy_stm32::Peripheral;

pub enum HeaterState {
    Off,
    Heating,
}

pub struct Heater<'d, T: GeneralInstance4Channel> {
    pwm: SimplePwm<'d, T>,
    state: HeaterState,
}

impl<'d, T: GeneralInstance4Channel> Heater<'d, T> {
    pub fn new(
        tim: impl Peripheral<P = T> + 'd,
        pwm_pin: Option<PwmPin<'d, T, Ch1>>,
        freq: Hertz,
    ) -> Self {
        let mut pwm = SimplePwm::new(
            tim,
            pwm_pin,
            None,
            None,
            None,
            freq,
            CountingMode::EdgeAlignedUp,
        );

        pwm.enable(Channel::Ch1);
        pwm.set_duty(Channel::Ch1, 0);

        Heater {
            pwm,
            state: HeaterState::Off,
        }
    }

    // Set PWM duty cycle (0-100%)
    pub fn heat(&mut self, duty_cycle: u8) {
        if duty_cycle > 100 {
            error!("ERR: duty_cycle > 100");
            return;
        }

        let duty = (self.pwm.get_max_duty() * duty_cycle as u32) / 100;
        self.pwm.set_duty(Channel::Ch1, duty);
        self.state = if duty_cycle == 0 {
            HeaterState::Off
        } else {
            HeaterState::Heating
        };
    }

    pub fn stop(&mut self) {
        self.pwm.set_duty(Channel::Ch1, 0);
        self.state = HeaterState::Off;
    }

    pub fn state(&self) -> &HeaterState {
        &self.state
    }
}
