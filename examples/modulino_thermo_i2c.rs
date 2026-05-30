//! # Arduino Modulino Thermo Example for Adafruit Feather ESP32-C6
//!
//! Reads temperature and humidity from the Arduino Modulino Thermo module (HS3003) over I2C.
//!
//! ## Hardware
//!
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Module:** Arduino Modulino Thermo (HS3003)
//!
//! ## Wiring with Qwiic/STEMMA QT
//!
//! Simply connect the Qwiic/STEMMA QT cable between the board and the Modulino Thermo.
//! The cable provides:
//! ```
//!      Modulino Thermo -> Adafruit Feather ESP32-C6
//! (black)  GND -> GND (Stemma GND)
//! (red)    VCC -> 3.3V (Stemma V+)
//! (yellow) SCL -> GPIO 18 (Stemma SCL)
//! (blue)   SDA -> GPIO 19 (Stemma SDA)
//! ```
//!
//! ## Run
//!
//! ```bash
//! cargo run --example modulino_thermo_i2c
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
use modulino::Thermo;

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut delay = Delay::new();

    info!("Initializing Arduino Modulino Thermo...");

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

    // Create Modulino Thermo driver
    // The HS3003 has a fixed address of 0x44
    let mut thermo = Thermo::new(i2c);

    info!(
        "Modulino Thermo initialized at address 0x{:02X}!",
        thermo.address()
    );
    info!("Starting measurements...");

    loop {
        // Read temperature and humidity
        // The read method requires a delay provider to wait for the measurement to complete
        match thermo.read(&mut delay) {
            Ok(measurement) => {
                info!(
                    "Temperature: {} °C, Humidity: {} %",
                    measurement.temperature, measurement.humidity
                );
            }
            Err(e) => {
                error!("Failed to read sensor: {:?}", Debug2Format(&e));
            }
        }

        // Wait 1 second before next measurement
        delay.delay_millis(1000);
    }
}
