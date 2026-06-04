//! # DHT11 Temperature & Humidity Sensor Example for Adafruit Feather ESP32-C6
//!
//! Demonstrates reading temperature and humidity from a DHT11 sensor using a single GPIO pin configured as a bidirectional (`Flex`) pin.
//!
//! ## Hardware
//!
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Sensor:** DHT11 Temperature & Humidity Sensor
//!
//! ## Hardware Wiring (Adafruit Feather ESP32-C6)
//!
//! | DHT11 Pin | GPIO | Silkscreen Label | Role             | Note                                                                 |
//! |-----------|------|------------------|------------------|----------------------------------------------------------------------|
//! | VCC       | -    | 3V               | Power (3.3V)     | Connect to 3V (3.3V output) pin                                     |
//! | GND       | -    | GND              | Ground           | Connect to GND pin                                                   |
//! | DATA      | 5    | A3 / IO5         | Bidirectional IO | Connect to GPIO5. Add a 4.7kΩ–10kΩ pull-up resistor to VCC if needed. |
//! | (Internal)| 20   | (internal)       | Peripheral Power | Powers the board's 3.3V peripheral rail                             |
//!
//! **Note on Pin Naming:** Look for the hole marked **A3** (also labeled IO5) — this is **GPIO 5** in code.
//!
//! **Pull-up Resistor:** The DHT11 data line requires a pull-up resistor (4.7kΩ to 10kΩ) connected to VCC.
//! If you are using a pre-assembled DHT11 module board, it likely already has this resistor. If you are
//! using a bare 4-pin DHT11 sensor, you must wire one externally.
//!
//! Run with `cargo run --example dht11 --release`.
//! Note: Due to timing sensitivity of the DHT11 protocol, you must run this example in **release** mode.

#![no_std]
#![no_main]

use dht_sensor::*;
use esp_hal::{
    delay::Delay,
    gpio::{DriveMode, Flex, InputConfig, Level, Output, OutputConfig, Pull},
    main,
};
use panic_rtt_target as _;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    // Initialize peripherals
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut delay = Delay::new();

    // Enable peripheral power rail (GPIO20 powers the 3.3V peripheral supply on this board)
    let _pwr = Output::new(peripherals.GPIO20, Level::High, OutputConfig::default());
    delay.delay_millis(100);

    defmt::println!("Initializing DHT11 sensor on GPIO 5 (A3)...");

    // Configure GPIO5 as a flexible, bidirectional pin with internal pull-up enabled
    let mut dht_pin = Flex::new(peripherals.GPIO5);

    // Enable both input and output buffers
    dht_pin.set_input_enable(true);
    dht_pin.set_output_enable(true);

    // Apply Pull::Up configuration so the line floats high when not driven low
    dht_pin.apply_input_config(&InputConfig::default().with_pull(Pull::Up));
    dht_pin.apply_output_config(&OutputConfig::default().with_drive_mode(DriveMode::OpenDrain));

    defmt::println!("DHT11 initialized. Starting reading loop (every 2 seconds)...");

    loop {
        // Perform a blocking read of the DHT11 sensor
        match dht11::blocking::read(&mut delay, &mut dht_pin) {
            Ok(reading) => {
                defmt::println!(
                    "Temperature: {}°C | Humidity: {}%",
                    reading.temperature,
                    reading.relative_humidity
                );
            }
            Err(e) => {
                // We print errors as info/warn since transient errors are common with DHT11 sensors
                match e {
                    DhtError::Timeout => {
                        defmt::println!(
                            "Error: Reading timed out. Verify wiring and ensure running in --release mode."
                        );
                    }
                    DhtError::ChecksumMismatch => {
                        defmt::println!(
                            "Error: Checksum mismatch. The data might have been corrupted."
                        );
                    }
                    _ => {
                        defmt::println!("Error reading sensor: {:?}", defmt::Debug2Format(&e));
                    }
                }
            }
        }

        // The DHT11 is slow and should not be polled more than once every 2 seconds
        delay.delay_millis(2000);
    }
}
