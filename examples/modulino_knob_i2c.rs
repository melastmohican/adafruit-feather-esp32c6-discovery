//! # Arduino Modulino Knob Example for Adafruit Feather ESP32-C6
//!
//! Reads rotary encoder value and button state from the Arduino Modulino Knob module over I2C.
//!
//! ## Hardware
//!
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Module:** Arduino Modulino Knob (Rotary Encoder)
//!
//! ## Wiring with Qwiic/STEMMA QT
//!
//! Simply connect the Qwiic/STEMMA QT cable between the board and the Modulino Knob.
//! The cable provides:
//! ```
//!      Modulino Knob -> Adafruit Feather ESP32-C6
//! (black)  GND -> GND (Stemma GND)
//! (red)    VCC -> 3.3V (Stemma V+)
//! (yellow) SCL -> GPIO 18 (Stemma SCL)
//! (blue)   SDA -> GPIO 19 (Stemma SDA)
//! ```
//!
//! ## Run
//!
//! ```bash
//! cargo run --example modulino_knob_i2c
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
use modulino::Knob;

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    info!("Initializing Arduino Modulino Knob...");

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

    // Create Modulino Knob driver
    let mut knob = match Knob::new(i2c) {
        Ok(k) => k,
        Err(e) => {
            error!("Failed to initialize Modulino Knob: {:?}", Debug2Format(&e));
            loop {
                delay.delay_millis(1000);
            }
        }
    };

    info!(
        "Modulino Knob initialized at address 0x{:02X}!",
        knob.address()
    );

    // Set range to 0-100 (e.g. for volume)
    knob.set_range(0, 100);
    // Reset starting value to 50
    if let Err(e) = knob.set_value(50) {
        error!("Failed to set initial value: {:?}", Debug2Format(&e));
    }

    info!("Turn the knob! (Range 0-100)");

    let mut prev_value = knob.value();
    let mut prev_pressed = knob.pressed();

    loop {
        // Poll for updates
        match knob.update() {
            Ok(_changed) => {
                let current_value = knob.value();
                let current_pressed = knob.pressed();

                if current_value != prev_value {
                    info!("Value: {}", current_value);
                    prev_value = current_value;
                }

                if current_pressed != prev_pressed {
                    if current_pressed {
                        info!("Button Pressed!");
                    } else {
                        info!("Button Released");
                    }
                    prev_pressed = current_pressed;
                }
            }
            Err(e) => {
                error!("Failed to update knob: {:?}", Debug2Format(&e));
            }
        }

        // Poll interval
        delay.delay_millis(20);
    }
}
