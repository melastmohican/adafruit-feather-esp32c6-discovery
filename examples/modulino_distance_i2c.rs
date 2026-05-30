//! # Arduino Modulino Distance Example for Adafruit Feather ESP32-C6
//!
//! Reads distance from the Arduino Modulino Distance module (VL53L4CD) over I2C.
//!
//! ## Hardware
//!
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Module:** Arduino Modulino Distance (VL53L4CD)
//!
//! ## Wiring with Qwiic/STEMMA QT
//!
//! Simply connect the Qwiic/STEMMA QT cable between the board and the Modulino Distance.
//! The cable provides:
//! ```
//!      Modulino Distance -> Adafruit Feather ESP32-C6
//! (black)  GND    -> GND (Stemma GND)
//! (red)    VCC    -> 3.3V (Stemma V+)
//! (yellow) SCL    -> GPIO 18 (Stemma SCL)
//! (blue)   SDA    -> GPIO 19 (Stemma SDA)
//! ```
//!
//! ## Run
//!
//! ```bash
//! cargo run --example modulino_distance_i2c
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
use modulino::Distance;

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut delay = Delay::new();

    info!("Initializing Arduino Modulino Distance...");

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

    // Create Modulino Distance driver
    let mut distance = Distance::new(i2c);

    // Initialize the sensor (loads firmware, sets tuning)
    info!("Waiting for sensor boot and loading firmware...");
    if let Err(e) = distance.init(&mut delay) {
        error!(
            "Failed to initialize Modulino Distance: {:?}",
            Debug2Format(&e)
        );
        loop {
            delay.delay_millis(1000);
        }
    }

    info!(
        "Modulino Distance initialized at address 0x{:02X}!",
        distance.address()
    );

    // Start continuous ranging
    if let Err(e) = distance.start_ranging() {
        error!("Failed to start ranging: {:?}", Debug2Format(&e));
    }
    info!("Ranging started...");

    loop {
        // Check if data is ready
        match distance.data_ready() {
            Ok(ready) => {
                if ready {
                    // Read distance
                    match distance.read_distance() {
                        Ok(Some(mm)) => {
                            info!("Distance: {} mm", mm);
                        }
                        Ok(None) => {
                            // If None is returned, check raw status if possible
                            if let Ok(status) = distance.read_range_status() {
                                info!("Invalid measurement (Status: {})", status);
                            } else {
                                info!("Invalid measurement");
                            }
                        }
                        Err(e) => {
                            error!("Failed to read distance: {:?}", Debug2Format(&e));
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to check data ready: {:?}", Debug2Format(&e));
            }
        }

        // Poll interval - poll every 500ms to reduce output frequency
        delay.delay_millis(500);
    }
}
