//! # Arduino Modulino Joystick Example for Adafruit Feather ESP32-C6
//!
//! Reads joystick position and button state over I2C and prints values via RTT.
//!
//! ## Hardware
//!
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Module:** Arduino Modulino Joystick
//!
//! ## Wiring with Qwiic/STEMMA QT
//!
//! Simply connect the Qwiic/STEMMA QT cable between the board and the Modulino Joystick.
//! The cable provides:
//! ```
//!      Modulino Joystick -> Adafruit Feather ESP32-C6
//! (black)  GND -> GND (Stemma GND)
//! (red)    VCC -> 3.3V (Stemma V+)
//! (yellow) SCL -> GPIO 18 (Stemma SCL)
//! (blue)   SDA -> GPIO 19 (Stemma SDA)
//! ```
//!
//! ## Run
//!
//! ```bash
//! cargo run --example modulino_joystick_i2c
//! ```

#![no_std]
#![no_main]

use defmt::{Debug2Format, error, info};
use esp_hal::{
    delay::Delay,
    i2c::master::{Config as I2cConfig, I2c},
    main,
    time::Rate,
};
use panic_rtt_target as _;

esp_bootloader_esp_idf::esp_app_desc!();

use modulino::Joystick;

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    info!("Initializing Arduino Modulino Joystick...");

    // Power on the I2C / NeoPixel port (GPIO 20)
    info!("Enabling I2C / NeoPixel Power (GPIO 20)");
    let _pwr = esp_hal::gpio::Output::new(
        peripherals.GPIO20,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );

    // Give the hardware a moment to boot up after receiving power
    delay.delay_millis(50);

    // Configure I2C pins
    let sda = peripherals.GPIO19;
    let scl = peripherals.GPIO18;

    // Create I2C peripheral
    let i2c_config = I2cConfig::default().with_frequency(Rate::from_khz(100));
    let i2c = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(sda)
        .with_scl(scl);

    let mut joystick = match Joystick::new(i2c) {
        Ok(j) => j,
        Err(e) => {
            error!("Failed to init Joystick: {:?}", Debug2Format(&e));
            loop {
                delay.delay_millis(1000);
            }
        }
    };

    info!(
        "Joystick initialized at address 0x{:02X}",
        joystick.address()
    );

    loop {
        if let Err(e) = joystick.update() {
            error!("Joystick update error: {:?}", Debug2Format(&e));
        } else {
            let (x, y) = joystick.position();
            let btn = joystick.button_pressed();
            let angle = joystick.angle();
            let mag = joystick.magnitude();
            info!(
                "Pos: ({}, {}), Button: {}, Angle: {}, Mag: {}",
                x,
                y,
                btn,
                Fmt(angle),
                Fmt(mag)
            );
        }
        delay.delay_millis(200);
    }
}

/// Helper struct for formatting floating-point numbers in `defmt` logs.
pub struct Fmt(pub f32);

impl defmt::Format for Fmt {
    fn format(&self, f: defmt::Formatter) {
        // Multiplier for 2 decimal places
        const PRECISION: f32 = 100.0;

        let scaled = (self.0 * PRECISION) as i32;
        let int = scaled / 100;
        let frac = (scaled % 100).abs();

        // Handle negative sign correctly for values between -1.0 and 0.0
        if scaled < 0 && int == 0 {
            defmt::write!(f, "-0.{:02}", frac);
        } else {
            defmt::write!(f, "{}.{:02}", int, frac);
        }
    }
}
