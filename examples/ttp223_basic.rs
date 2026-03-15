//! # TTP223 Digital Capacitive Touch Sensor Basic Example for Adafruit Feather ESP32-C6
//!
//! Minimal touch detection — prints a message on touch and release.
//! For debouncing and tap/long-press detection see `ttp223_touch`.
//!
//! ## Hardware
//!
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Sensor:** TTP223 Digital Capacitive Touch Sensor
//!
//! ## Hardware Wiring (Adafruit Feather ESP32-C6)
//!
//! | TTP223 Pin | GPIO | Silkscreen Label | Role             |
//! |------------|------|------------------|------------------|
//! | VCC        | -    | 3V               | Power (3.3V)     |
//! | GND        | -    | GND              | Ground           |
//! | SIG        | 5    | A3 / IO5         | Touch Signal In  |
//! | (Internal) | 20   | (internal)       | Peripheral Power |
//!
//! **Note on Pin Naming:** On this board, silkscreen labels often differ from
//! GPIO numbers. Look for the hole marked **A3** (also labeled IO5) — this is
//! **GPIO 5** in the code. Do not use the hole marked "D3".
//!
//! ## TTP223 Configuration (Default)
//! - **A (Solder Pad):** Open (Active-HIGH)
//! - **B (Solder Pad):** Open (Momentary)
//!
//! Run with `cargo run --example ttp223_basic`.

#![no_std]
#![no_main]

use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    main,
};
use panic_rtt_target as _;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();
    // Initialize peripherals
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    // Enable peripheral power rail (GPIO20 powers the 3.3V peripheral supply on this board)
    let _pwr = Output::new(peripherals.GPIO20, Level::High, OutputConfig::default());
    delay.delay_millis(100);

    // Pull::None — confirmed working; Pull::Down interferes with the TTP223 output
    let touch_sensor = Input::new(
        peripherals.GPIO5,
        InputConfig::default().with_pull(Pull::None),
    );

    defmt::println!("TTP223 Sensor Initialized. Waiting for touch...");

    loop {
        // TTP223 OUT pin is HIGH when the pad is touched
        if touch_sensor.is_high() {
            defmt::println!("Touch Detected!");

            // Wait until touch is released to avoid flooding logs
            while touch_sensor.is_high() {
                delay.delay_millis(10);
            }
            defmt::println!("Touch Released.");
        }

        // Small delay to prevent tight-looping
        delay.delay_millis(50);
    }
}
