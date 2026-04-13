//! # BME280 Temperature/Humidity/Pressure Sensor Example for Adafruit Feather ESP32-C6
//!
//! Reads temperature, humidity, and atmospheric pressure from a BME280 sensor over I2C.
//!
//! ## Hardware
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Sensor:** Pimoroni Enviro+ FeatherWing (BME280)
//! - **Connection:** FeatherWing Headers
//!
//! ## Pin Mapping (Feather C6)
//! - **SDA:** GPIO 18
//! - **SCL:** GPIO 19
//! - **PWR:** GPIO 20 (Must be HIGH to power the headers/Stemma port)
//!
//! ## I2C Address
//!
//! The BME280 can have two I2C addresses:
//! - 0x76 (SDO pin to GND) - use `BME280::new_primary()`  **← Pimoroni default**
//! - 0x77 (SDO pin to VCC) - use `BME280::new_secondary()`
//!
//! The Pimoroni BME280 uses address 0x76 by default.
//!
//! Run with `cargo run --example enviro_bme280_i2c`.

#![no_std]
#![no_main]

use bme280::i2c::BME280;
use defmt::info;
use esp_hal::{
    delay::Delay,
    i2c::master::{Config as I2cConfig, I2c},
    main,
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
    // This is required on the Feather C6 to power the Stemma QT port and headers
    let _pwr = esp_hal::gpio::Output::new(
        peripherals.GPIO20,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );

    // Give hardware a moment to boot
    delay.delay_millis(50);

    // Configure I2C pins for Adafruit Feather ESP32-C6
    let sda = peripherals.GPIO19;
    let scl = peripherals.GPIO18;

    // Create I2C peripheral with default configuration (100kHz)
    // BME280 supports up to 400kHz (Fast mode)
    let i2c = I2c::new(peripherals.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(sda)
        .with_scl(scl);

    // Initialize BME280 sensor
    // Pimoroni BME280 uses 0x76 address (new_primary)
    let mut bme280 = BME280::new_primary(i2c);

    // Initialize the sensor
    match bme280.init(&mut delay) {
        Ok(_) => info!("BME280 initialized successfully!"),
        Err(_) => {
            info!("Failed to initialize BME280!");
            info!("Check wiring and I2C address");
            info!("Pimoroni BME280 uses 0x76 (new_primary)");
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
