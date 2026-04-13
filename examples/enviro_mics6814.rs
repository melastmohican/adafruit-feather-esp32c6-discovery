//! # MICS6814 Analog Gas Sensor Example for Adafruit Feather ESP32-C6
//!
//! Reads three analog gas channels (oxidising, reducing, NH3) from the
//! MICS6814 on the Pimoroni Enviro+ FeatherWing.
//!
//! ## Hardware
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Sensor:** Pimoroni Enviro+ FeatherWing (MICS6814)
//!
//! ## Pin Mapping (Feather C6)
//! - **NH3 (A0):** GPIO 0
//! - **RED (A1):** GPIO 1
//! - **OX  (A2):** GPIO 6 (Note: Shared with LCD Chip Select!)
//! - **EN  (D4):** GPIO 4 (Heater Enable)
//! - **PWR:** GPIO 20 (Power Enable)
//!
//! The conversion formula mirrors Pimoroni's Python implementation:
//! R = 56000 / ((ADC_MAX / raw) - 1)
//!
//! Run with:
//! ```bash
//! cargo run --example enviro_mics6814
//! ```

#![no_std]
#![no_main]

use defmt::info;
use esp_hal::{
    analog::adc::{Adc, AdcConfig, Attenuation},
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    main,
};
use panic_rtt_target as _;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    info!("Initializing MICS6814 example");

    // Power on the I2C / NeoPixel port (GPIO 20)
    let _pwr = esp_hal::gpio::Output::new(
        peripherals.GPIO20,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );

    let delay = Delay::new();

    let mut en = Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default());
    en.set_high();
    info!("Enable pin set high (GPIO4)");

    let mut adc_config = AdcConfig::new();

    let mut pin_ox = adc_config.enable_pin(peripherals.GPIO6, Attenuation::_11dB);
    let mut pin_red = adc_config.enable_pin(peripherals.GPIO1, Attenuation::_11dB);
    let mut pin_nh3 = adc_config.enable_pin(peripherals.GPIO0, Attenuation::_11dB);

    let mut adc = Adc::new(peripherals.ADC1, adc_config);
    fn adc_to_resistance(raw: u32, adc_max: u32) -> Option<f32> {
        if raw == 0 || raw >= adc_max {
            // Match Pimoroni Python: on out-of-range or division-by-zero,
            // return numeric zero instead of None.
            return Some(0.0);
        }
        let denom = (adc_max as f32) / (raw as f32) - 1.0;
        if denom <= 0.0 {
            return Some(0.0);
        }
        Some(56000.0 / denom)
    }

    info!("Starting readings...");

    // Use 12-bit ADC scale by default for ESP targets.
    // If you're using a platform or HAL that provides 16-bit ADC values
    // (Pimoroni Python uses 65535), change `adc_max` to `65535u32` below.
    let adc_max: u32 = 4095u32;

    loop {
        let raw_ox_u16: u16 = adc.read_oneshot(&mut pin_ox).unwrap_or_default();
        let raw_red_u16: u16 = adc.read_oneshot(&mut pin_red).unwrap_or_default();
        let raw_nh3_u16: u16 = adc.read_oneshot(&mut pin_nh3).unwrap_or_default();

        let raw_ox: u32 = raw_ox_u16 as u32;
        let raw_red: u32 = raw_red_u16 as u32;
        let raw_nh3: u32 = raw_nh3_u16 as u32;

        let r_ox = adc_to_resistance(raw_ox, adc_max);
        let r_red = adc_to_resistance(raw_red, adc_max);
        let r_nh3 = adc_to_resistance(raw_nh3, adc_max);

        let v_ox = (raw_ox as f32) / (adc_max as f32) * 3.3;
        let v_red = (raw_red as f32) / (adc_max as f32) * 3.3;
        let v_nh3 = (raw_nh3 as f32) / (adc_max as f32) * 3.3;

        let r = r_ox.unwrap_or(0.0);
        let ox_int = r as i32;
        let ox_frac = (((r.abs() % 1.0) * 1000.0) as u32).min(999);
        let v_ox_int = v_ox as u32;
        let v_ox_frac = (((v_ox % 1.0) * 100.0) as u32).min(99);
        info!(
            "OX raw={} V={}.{:02}V Oxidising: {}.{:03} Ohms",
            raw_ox, v_ox_int, v_ox_frac, ox_int, ox_frac
        );

        let r = r_red.unwrap_or(0.0);
        let red_int = r as i32;
        let red_frac = (((r.abs() % 1.0) * 1000.0) as u32).min(999);
        let v_red_int = v_red as u32;
        let v_red_frac = (((v_red % 1.0) * 100.0) as u32).min(99);
        info!(
            "RED raw={} V={}.{:02}V Reducing:  {}.{:03} Ohms",
            raw_red, v_red_int, v_red_frac, red_int, red_frac
        );

        let r = r_nh3.unwrap_or(0.0);
        let nh3_int = r as i32;
        let nh3_frac = (((r.abs() % 1.0) * 1000.0) as u32).min(999);
        let v_nh3_int = v_nh3 as u32;
        let v_nh3_frac = (((v_nh3 % 1.0) * 100.0) as u32).min(99);
        info!(
            "NH3 raw={} V={}.{:02}V NH3:       {}.{:03} Ohms",
            raw_nh3, v_nh3_int, v_nh3_frac, nh3_int, nh3_frac
        );

        delay.delay_millis(1000u32);
    }
}
