use embassy_stm32::time::Hertz;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{Ch2, Ch3, PwmPin, SimplePwm};
use embassy_stm32::timer::{Channel, GeneralInstance4Channel};
use embassy_stm32::Peripheral;
use embassy_time::Timer;

const INTERVAL_MS: u64 = 1000;
const DUTY_CYCLE: u64 = 50; // must be [0,100]

#[derive(Debug, PartialEq)]
pub enum ThermalState {
    Off,
    Heating,
    Cooling,
}

pub struct PeltierController<'d, T: GeneralInstance4Channel> {
    pwm: SimplePwm<'d, T>,
    state: ThermalState,
}

impl<'d, T: GeneralInstance4Channel> PeltierController<'d, T> {
    pub fn new(
        tim: impl Peripheral<P = T> + 'd,
        ch2_pin: Option<PwmPin<'d, T, Ch2>>,
        ch3_pin: Option<PwmPin<'d, T, Ch3>>,
    ) -> Self {
        let mut pwm = SimplePwm::new(
            tim,
            None,
            ch2_pin, // PA9 as Channel 2
            ch3_pin, // PA10 as Channel 3
            None,
            Hertz(50_000),
            CountingMode::EdgeAlignedUp,
        );

        pwm.enable(Channel::Ch2);
        pwm.enable(Channel::Ch3);
        pwm.set_duty(Channel::Ch2, 0);
        pwm.set_duty(Channel::Ch3, 0);

        Self {
            pwm,
            state: ThermalState::Off,
        }
    }

    pub async fn heat(&mut self) {
        if self.state == ThermalState::Cooling {
            self.stop();
        }

        let duty = (self.pwm.get_max_duty() * DUTY_CYCLE as u32) / 100;
        self.pwm.set_duty(Channel::Ch2, duty);
        self.pwm.set_duty(Channel::Ch3, 0);

        self.state = if DUTY_CYCLE == 0 {
            ThermalState::Off
        } else {
            ThermalState::Heating
        };
        Timer::after_millis(INTERVAL_MS).await;
        self.stop();
    }

    pub async fn cool(&mut self) {
        if self.state == ThermalState::Heating {
            self.stop();
        }

        let duty = (self.pwm.get_max_duty() * DUTY_CYCLE as u32) / 100;
        self.pwm.set_duty(Channel::Ch2, 0);
        self.pwm.set_duty(Channel::Ch3, duty);

        self.state = if DUTY_CYCLE == 0 {
            ThermalState::Off
        } else {
            ThermalState::Cooling
        };
        Timer::after_millis(INTERVAL_MS).await;
        self.stop();
    }

    pub fn stop(&mut self) {
        self.pwm.set_duty(Channel::Ch2, 0);
        self.pwm.set_duty(Channel::Ch3, 0);
        self.state = ThermalState::Off;
    }

    pub fn state(&self) -> &ThermalState {
        &self.state
    }
}
