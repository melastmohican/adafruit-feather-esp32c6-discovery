#![no_std]
#![no_main]

//! # Arduino Modulino Light Example for Adafruit Feather ESP32-C6
//!
//! This example uses the **modulino** library: https://crates.io/crates/modulino
//!
//! It reads RGB, IR, and Lux values from the Modulino Light sensor (LTR-381RGB)
//! over I2C and prints them using `defmt`.
//!
//! ## Hardware
//!
//! - **Module:** Arduino Modulino Light
//! - **Connection:** Qwiic/STEMMA QT cable (I2C)
//! - **I2C Address:** 0x53 (7-bit)
//!
//! ## Wiring with Qwiic/STEMMA QT on Adafruit Feather ESP32-C6
//!
//! ```
//! Modulino Light -> Feather ESP32-C6
//! (black)  GND -> GND
//! (red)    VCC -> 3.3V
//! (yellow) SCL -> SCL (GPIO 18)
//! (blue)   SDA -> SDA (GPIO 19)
//!
//! NOTE: GPIO 20 (Power Enable) must be HIGH to power the Stemma QT port.
//! ```
//!
//! Run with `cargo run --example modulino_light_i2c`.

use defmt::{Debug2Format, error, info};
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    i2c::master::{Config as I2cConfig, I2c},
    main,
    time::Rate,
};
use modulino::Light;
use panic_rtt_target as _;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    info!("Initializing Arduino Modulino Light...");

    // Note: GPIO 20 MUST be HIGH to power the Stemma QT port on the Feather ESP32-C6
    info!("Enabling Stemma/I2C Power (GPIO 20)");
    let _pwr = Output::new(peripherals.GPIO20, Level::High, OutputConfig::default());
    delay.delay_millis(500); // Wait for sensor to power up

    let i2c_config = I2cConfig::default().with_frequency(Rate::from_khz(100));
    let i2c = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(peripherals.GPIO19)
        .with_scl(peripherals.GPIO18);

    let mut light = Light::new(i2c);

    // init() sets 18x Gain, 16-bit Resolution, and 25ms Rate (Arduino defaults)
    if let Err(e) = light.init() {
        error!("Failed to init Light sensor: {:?}", Debug2Format(&e));
        loop {
            delay.delay_millis(1000);
        }
    }

    info!("Light sensor initialized!");

    loop {
        // Read all channels and calculate Lux/Color
        match light.read() {
            Ok(meas) => {
                let color = meas.color_name();
                info!(
                    "R: {}, G: {}, B: {}, IR: {}, Raw Lux: {}, Lux: {}, Color: {}",
                    meas.red,
                    meas.green,
                    meas.blue,
                    meas.ir,
                    meas.raw_lux,
                    Fmt(meas.lux),
                    color
                );
            }
            Err(e) => {
                error!("Light read error: {:?}", Debug2Format(&e));
            }
        }

        delay.delay_millis(500);
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
