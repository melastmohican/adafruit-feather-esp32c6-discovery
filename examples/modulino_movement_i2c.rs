//! # Arduino Modulino Movement Example for Adafruit Feather ESP32-C6
//!
//! Reads accelerometer and gyroscope data from the Arduino Modulino Movement module (LSM6DSOX) over I2C.
//!
//! ## Hardware
//!
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Module:** Arduino Modulino Movement
//!
//! ## Wiring with Qwiic/STEMMA QT
//!
//! Simply connect the Qwiic/STEMMA QT cable between the board and the Modulino Movement.
//! The cable provides:
//! ```
//!      Modulino Movement -> Adafruit Feather ESP32-C6
//! (black)  GND -> GND (Stemma GND)
//! (red)    VCC -> 3.3V (Stemma V+)
//! (yellow) SCL -> GPIO 18 (Stemma SCL)
//! (blue)   SDA -> GPIO 19 (Stemma SDA)
//! ```
//!
//! ## Run
//!
//! ```bash
//! cargo run --example modulino_movement_i2c
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

// Import from modulino library
use modulino::Movement;

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    info!("Initializing Arduino Modulino Movement...");

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

    // Create Modulino Movement driver
    let mut movement = match Movement::new(i2c) {
        Ok(m) => m,
        Err(e) => {
            error!(
                "Failed to initialize Modulino Movement: {:?}",
                Debug2Format(&e)
            );
            loop {
                delay.delay_millis(1000);
            }
        }
    };

    info!(
        "Modulino Movement initialized at address 0x{:02X}!",
        movement.address()
    );
    info!("Starting measurements...");

    loop {
        // Read accelerometer and gyroscope values
        match movement.acceleration() {
            Ok(values) => {
                info!(
                    "Accel: x={} g, y={} g, z={} g",
                    Fmt(values.x),
                    Fmt(values.y),
                    Fmt(values.z)
                );
            }
            Err(e) => {
                error!("Failed to read acceleration: {:?}", Debug2Format(&e));
            }
        }

        match movement.gyro() {
            Ok(values) => {
                info!(
                    "Gyro:  x={} dps, y={} dps, z={} dps",
                    Fmt(values.x),
                    Fmt(values.y),
                    Fmt(values.z)
                );
            }
            Err(e) => {
                error!("Failed to read gyro: {:?}", Debug2Format(&e));
            }
        }

        // Wait 500ms before next measurement
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
