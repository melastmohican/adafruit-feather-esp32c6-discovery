//! # STHS34PF80 IR Presence / Motion Sensor Example for Adafruit Feather ESP32-C6
//!
//! Reads presence, motion, and temperature data from an STHS34PF80 sensor over I2C.
//!
//! ## Hardware
//!
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Sensor:** Adafruit STHS34PF80 IR Presence / Motion Sensor
//! - **Connection:** Qwiic/STEMMA QT cable (I2C)
//! - **I2C Address:** 0x5A (default for STHS34PF80)
//!
//! ## Wiring with Qwiic/STEMMA QT
//!
//! Simply connect the Qwiic/STEMMA QT cable between the board and the sensor.
//! The cable provides:
//! ```
//!      STHS34PF80 -> Adafruit Feather ESP32-C6
//! (black)  GND    -> GND (Stemma GND)
//! (red)    VCC    -> 3.3V (Stemma V+)
//! (yellow) SCL    -> GPIO 18 (Stemma SCL)
//! (blue)   SDA    -> GPIO 19 (Stemma SDA)
//! ```
//!
//! Run with `cargo run --example sths34pf80_i2c`.
//!
//! ## Understanding the Data
//!
//! - **Presence & Motion:** Internal algorithm outputs. Large values (e.g., > 1000) indicate
//!   significant detection. Negative values indicate a decrease in the detected signal
//!   (e.g., something moving away).
//! - **Raw Obj IR:** The raw infrared radiant power intensity. This is the value that
//!   the internal algorithms use to determine presence and motion.
//!
//! ## Expected Output
//!
//! ```text
//! [INFO ] Presence: 152 | Motion: -24 | Raw Obj IR: 2989 (Intensity)
//! [INFO ] Presence: 562 | Motion: 392 | Raw Obj IR: 2568 (Intensity)
//! [INFO ] Presence: 572 | Motion: 390 | Raw Obj IR: 2459 (Intensity)
//! ```

#![no_std]
#![no_main]

use defmt::{error, info};
use esp_hal::{
    delay::Delay,
    i2c::master::{Config as I2cConfig, I2c},
    main,
    time::Rate,
};
use panic_rtt_target as _;
use sths34pf80::Sths34pf80;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    info!("Initializing STHS34PF80 sensor...");

    // Power on the I2C / NeoPixel port (GPIO 20)
    info!("Enabling I2C / NeoPixel Power (GPIO 20)");
    let _pwr = esp_hal::gpio::Output::new(
        peripherals.GPIO20,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );

    // Give the hardware a moment to boot up after receiving power
    delay.delay_millis(500);

    // Configure I2C pins
    let sda = peripherals.GPIO19;
    let scl = peripherals.GPIO18;

    // Create I2C peripheral
    let i2c_config = I2cConfig::default().with_frequency(Rate::from_khz(100));
    let i2c = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(sda)
        .with_scl(scl);

    // Initialize STHS34PF80 sensor
    // Default I2C address is 0x5A
    let mut sensor = Sths34pf80::new(i2c, delay);

    // Initialize the sensor with default configuration
    match sensor.initialize() {
        Ok(_) => info!("STHS34PF80 initialized successfully!"),
        Err(e) => {
            error!(
                "Failed to initialize STHS34PF80: {:?}",
                defmt::Debug2Format(&e)
            );
            loop {
                delay.delay_millis(1000);
            }
        }
    }

    info!("Starting measurements...");

    loop {
        let delay = Delay::new();
        // Read measurements individually for clearer control
        // Note: get_presence and get_tmotion return algorithm scores from the sensor
        let presence = sensor.get_presence().unwrap_or(0);
        let motion = sensor.get_tmotion().unwrap_or(0);

        // Note: get_temperature returns the raw i16 Object IR Intensity.
        // It's called "temperature" in the crate but it's actually unscaled radiant power.
        // This matches the "Obj" column in the Arduino examples.
        let obj_raw = sensor.get_temperature().unwrap_or(0);

        // Note: The crate (v0.1.12) does not currently have a public method
        // to read Ambient Temperature (register 0x28).

        info!(
            "Presence: {} | Motion: {} | Raw Obj IR: {} (Intensity)",
            presence, motion, obj_raw,
        );

        // Wait 500ms between measurements
        delay.delay_millis(500);
    }
}
