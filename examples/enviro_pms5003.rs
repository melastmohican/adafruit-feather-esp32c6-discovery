//! # PMS5003 PM2.5 Air Quality Sensor (UART) Example
//!
//! Reads particulate matter data from a PMS5003 sensor over UART (9600 8N1)
//! and prints PM1.0 / PM2.5 / PM10 concentrations via RTT/defmt.
//!
//! ## Hardware
//!
//! - **Board:** Rust ESP Board (ESP32-C3-DevKit-RUST-1)
//! - **Sensor:** Adafruit PM2.5 Air Quality Sensor (PMS5003) with breadboard adapter
//! - **FeatherWing:** Pimoroni Enviro+ FeatherWing (if used to mount the PMS5003)
//!
//! ## Wiring (recommended for this example)
//!
//! Wire the PMS5003 adapter to the Rust ESP Board as follows:
//!
//! ```text
//! PMS5003 TXD -> MCU GPIO21  (connects to MCU RX)
//! PMS5003 RXD -> MCU GPIO20  (connects to MCU TX)
//! PMS5003 VCC -> 5V (or as adapter requires)
//! PMS5003 GND -> GND
//! ```
//!
//! Note: the board silkscreen may label header pins `IO21/TX` and `IO20/RX`.
//! The authoritative mapping is the SoC gpio names used in code (`peripherals.GPIO21` / `peripherals.GPIO20`).
//! On this board the working wiring is: sensor TX -> `GPIO21`, sensor RX -> `GPIO20`.
//!
//! ## Notes
//!
//! - PMS5003 default UART settings: 9600 baud, 8 data bits, no parity, 1 stop bit.
//! - The Adafruit adapter typically requires 5V VCC and includes level shifting — verify before connecting directly to 3.3V MCUs.
//! - If you see no data, try swapping TX/RX assignments in the example and re-run.
//!
//! ## Run
//!
//! ```bash
//! cargo run --example pms5003
//! ```

#![no_std]
#![no_main]

use defmt::println;
use esp_hal::{
    Config as HalConfig,
    delay::Delay,
    main,
    uart::{Config as UartConfig, Uart},
};
use panic_rtt_target as _;
use pmsx003::PmsX003Sensor;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(HalConfig::default());

    // Simple delay
    let delay = Delay::new();

    let config = UartConfig::default().with_baudrate(9_600);

    let tx = peripherals.GPIO20; // MCU TX -> sensor RX
    let rx = peripherals.GPIO21; // MCU RX <- sensor TX

    println!("Using MCU TX=GPIO20 (to sensor RX) and MCU RX=GPIO21 (from sensor TX)");

    // Create UART and bind pins
    let uart = Uart::new(peripherals.UART1, config)
        .unwrap()
        .with_tx(tx)
        .with_rx(rx);

    // Create sensor instance - esp-hal's Uart implements embedded-io traits
    let mut sensor = PmsX003Sensor::new(uart);

    println!("PMS5003 sensor initialized");
    println!("Starting continuous readings...");

    loop {
        match sensor.read() {
            Ok(frame) => {
                println!("PM1.0: {} μg/m³", frame.pm1_0);
                println!("PM2.5: {} μg/m³", frame.pm2_5);
                println!("PM10:  {} μg/m³", frame.pm10);
                println!("---");
            }
            Err(e) => {
                println!("Error reading sensor: {:?}", defmt::Debug2Format(&e));
            }
        }

        delay.delay_millis(2000u32); // Read every 2 seconds
    }
}
