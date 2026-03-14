//! # BME280 Temperature/Humidity/Pressure Sensor Example for Adafruit Feather ESP32-C6
//!
//! Reads temperature, humidity, and atmospheric pressure from a BME280 sensor over I2C.
//!
//! ## Hardware
//!
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Sensor:** Adafruit BME280 Temperature Humidity Pressure Sensor
//! - **Connection:** Qwiic/STEMMA QT cable (I2C)
//! - **I2C Address:** 0x77 (default for Adafruit BME280)
//!
//! ## Wiring with Qwiic/STEMMA QT
//!
//! Simply connect the Qwiic/STEMMA QT cable between the board and the BME280 sensor.
//! The cable provides:
//! ```
//!      BME280 -> Adafruit Feather ESP32-C6
//! (black)  GND -> GND (Stemma GND)
//! (red)    VCC -> 3.3V (Stemma V+)
//! (yellow) SCL -> GPIO 18 (Stemma SCL)
//! (blue)   SDA -> GPIO 19 (Stemma SDA)
//! ```
//!
//! Run with `cargo run --example bme280_i2c`.

#![no_std]
#![no_main]

use bme280::i2c::BME280;
use defmt::info;
use esp_hal::{
    delay::Delay,
    i2c::master::{Config as I2cConfig, I2c},
    main,
    time::Rate,
};
use panic_rtt_target as _;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut delay = Delay::new();

    info!("Initializing BME280 sensor...");

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

    // Initialize BME280 sensor
    // Adafruit BME280 uses 0x77 address (new_secondary)
    // Use new_primary() for 0x76 address if needed
    let mut bme280 = BME280::new_secondary(i2c);

    // Initialize the sensor
    match bme280.init(&mut delay) {
        Ok(_) => info!("BME280 initialized successfully!"),
        Err(_) => {
            info!("Failed to initialize BME280!");
            info!("Check wiring and I2C address");
            info!("Adafruit BME280 uses 0x77 (new_secondary), others may use 0x76 (new_primary)");
            loop {
                delay.delay_millis(1000);
            }
        }
    }

    info!("Starting measurements...");

    loop {
        // Take a measurement
        match bme280.measure(&mut delay) {
            Ok(measurements) => {
                info!(
                    "Temperature: {}.{:02} °C | Humidity: {}.{:02} % | Pressure: {}.{:02} hPa",
                    measurements.temperature as i32,
                    ((measurements.temperature.abs() % 1.0) * 100.0) as u32,
                    measurements.humidity as u32,
                    ((measurements.humidity % 1.0) * 100.0) as u32,
                    (measurements.pressure / 100.0) as u32,
                    ((measurements.pressure / 100.0 % 1.0) * 100.0) as u32,
                );
            }
            Err(_) => {
                info!("Error reading BME280 sensor!");
            }
        }

        // Wait 1 second between measurements
        delay.delay_millis(1000);
    }
}
