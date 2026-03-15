//! # TTP223 Digital Capacitive Touch Sensor Example for Adafruit Feather ESP32-C6
//!
//! Monitors a TTP223 touch sensor with asymmetric debouncing and detects taps
//! vs long presses. Lights the onboard LED while the pad is held.
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
//! | (Internal) | 15   | LED              | Onboard User LED |
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
//! Run with `cargo run --example ttp223_touch`.

#![no_std]
#![no_main]

use defmt::info;
use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    main,
    time::Instant,
};
use panic_rtt_target as _;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    info!("Initializing TTP223 Touch Sensor Example (GPIO5 / A3)...");
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    // Initialize the User LED (GPIO 15)
    let mut led = Output::new(peripherals.GPIO15, Level::Low, OutputConfig::default());

    // Power on the I2C / NeoPixel / Stemma QT port (GPIO 20)
    // Often required for peripherals sharing the power rail
    let _pwr = Output::new(peripherals.GPIO20, Level::High, OutputConfig::default());
    delay.delay_millis(100);

    // Touch sensor input on GPIO5 (silkscreen: A3 / IO5)
    // Pull::None required — Pull::Down suppresses the TTP223 output signal.
    let touch_sensor = Input::new(
        peripherals.GPIO5,
        InputConfig::default().with_pull(Pull::None),
    );

    info!("Monitoring touch sensor on GPIO5 (A3 / IO5)...");
    info!("Asymmetric Debounce: 2ms ON, 50ms OFF");

    let mut last_raw_level = touch_sensor.level();
    let mut stable_level = last_raw_level;
    let mut last_raw_change_time = Instant::now();
    let mut touch_start_time: Option<Instant> = None;
    let mut long_press_reported = false;

    let on_debounce_duration = esp_hal::time::Duration::from_millis(2); // Fast trigger
    let off_debounce_duration = esp_hal::time::Duration::from_millis(50); // Hold through flickering
    let long_press_duration = esp_hal::time::Duration::from_millis(1000);

    loop {
        let current_raw_level = touch_sensor.level();
        let now = Instant::now();

        // 1. Detect Raw State Changes (for debouncing timer)
        if current_raw_level != last_raw_level {
            last_raw_level = current_raw_level;
            last_raw_change_time = now;
        }

        // 2. Asymmetric Debouncing Logic
        let duration_stable = now - last_raw_change_time;

        if stable_level == Level::Low {
            // Looking to switch to High (Touch)
            if current_raw_level == Level::High && duration_stable >= on_debounce_duration {
                stable_level = Level::High;
                info!("Sensor Touched!");
                led.set_high();
                touch_start_time = Some(now);
                long_press_reported = false;
            }
        } else {
            // Looking to switch to Low (Release)
            if current_raw_level == Level::Low && duration_stable >= off_debounce_duration {
                stable_level = Level::Low;
                info!("Sensor Released");
                led.set_low();
                if let Some(start) = touch_start_time {
                    let duration = now - start;
                    if duration < long_press_duration {
                        info!("Action: Tap ({}ms)", duration.as_millis());
                    }
                }
                touch_start_time = None;
            }
        }

        // 3. Gesture Detection (Long Press)
        let is_long_press = stable_level == Level::High
            && !long_press_reported
            && touch_start_time.is_some_and(|start| (now - start) >= long_press_duration);

        if is_long_press {
            info!("Action: Long Press!");
            long_press_reported = true;
        }

        delay.delay_millis(1);
    }
}
