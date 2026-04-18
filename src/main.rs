#![no_std]
#![no_main]

use defmt::info;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    i2c::master::{BusTimeout, Config as I2cConfig, I2c},
    main,
    rmt::Rmt,
    time::Rate,
};
use esp_hal_smartled::{SmartLedsAdapter, smart_led_buffer};
use panic_rtt_target as _;
use smart_leds::{
    SmartLedsWrite, brightness, gamma,
    hsv::{Hsv, hsv2rgb},
};

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(clippy::large_stack_frames)]
#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    info!("Initializing peripherals...");
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let delay = Delay::new();

    // Power on the NeoPixel (GPIO 20)
    info!("Powering on NeoPixel (GPIO 20)");
    let mut _pwr = Output::new(peripherals.GPIO20, Level::High, OutputConfig::default());

    // User LED (GPIO 15)
    let mut user_led = Output::new(peripherals.GPIO15, Level::Low, OutputConfig::default());

    // Initialize RMT for NeoPixel (GPIO 9)
    info!("Initializing RMT for NeoPixel (GPIO 9)");
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    let mut rmt_buffer = smart_led_buffer!(1);
    let mut led = SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO9, &mut rmt_buffer);

    // Initialize I2C for scanning (SDA: GPIO 19, SCL: GPIO 18 as per standard Feather ESP32-C6)
    info!("Initializing I2C0 (SDA: GPIO 19, SCL: GPIO 18)");
    let i2c_config = I2cConfig::default()
        .with_frequency(Rate::from_khz(400))
        .with_timeout(BusTimeout::BusCycles(100)); // Add a 100-cycle timeout to prevent indefinite hangs

    let mut i2c = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(peripherals.GPIO19)
        .with_scl(peripherals.GPIO18);

    info!("Adafruit Feather ESP32-C6 Factory Test (Rust) starting loop...");

    let mut color_step: u8 = 0;

    loop {
        // Blink user LED every ~250ms (25 steps * 10ms)
        if color_step.is_multiple_of(25) {
            user_led.toggle();
        }

        // Rainbow cycle NeoPixel
        let color = hsv2rgb(Hsv {
            hue: color_step,
            sat: 255,
            val: 255,
        });

        // Write to NeoPixel using the adapter
        led.write(gamma(brightness([color].iter().cloned(), 32)))
            .unwrap();

        color_step = color_step.wrapping_add(1);

        // Scan I2C every 256 steps
        if color_step == 0 {
            info!("Scanning I2C bus...");
            for address in 1..127 {
                // We use a 1-byte read to ping the address. The BusTimeout(100)
                // in the I2cConfig ensures it aborts quickly if no ACK/pullups are present.
                let _res = i2c.read(address, &mut [0u8]);
                if _res.is_ok() {
                    info!("Found I2C device at address: 0x{:02x}", address);
                }
            }
        }

        delay.delay_millis(10);
    }
}
