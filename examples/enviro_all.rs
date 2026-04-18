//! # Enviro All Dashboard for Adafruit Feather ESP32-C6
//!
//! A comprehensive all-in-one dashboard that cycles through all sensors on the
//! Pimoroni Enviro+ FeatherWing and displays them on the ST7735R LCD.
//!
//! ## Pages:
//! 1. **Climate**: Temperature, Humidity, Pressure (BME280)
//! 2. **Light**: Ambient Light and Proximity (LTR-559)
//! 3. **Gases**: OX, RED, and NH3 resistances (MICS6814)
//! 4. **Particles**: PM1.0, PM2.5, PM10 (PMS5003 UART)
//!
//! ## Technical Rationale: The (1, 26) Offsets
//! Centering a 160x80 glass within a 132x162 controller buffer requires
//! precise offsets to avoid internal memory wrapping.
//!
//! ## The GPIO 6 "Dual Ownership" Strategy
//! GPIO 6 is physically shared between LCD_CS and the OX Gas Sensor. In Rust/esp-hal,
//! this creates an ownership conflict. We resolve this by creating two handles to
//! the physical hardware at boot (via unsafe duplication). We manage the conflict
//! manually by ensuring the Display is deselected (CS High) during ADC sensing.
//!
//! ## Deep Particle Hunting
//! UART buffers often contain stale data from when the board was busy with other sensors.
//! We implement a "Backlog Clear" and a 20-retry "Hunting" loop to ensure the Particle
//! data on screen is fresh and non-zero.
//!
//! Run with `cargo run --example enviro_all`.

#![no_std]
#![no_main]

use core::cell::RefCell;
use defmt::info;
use display_interface_spi::SPIInterface;
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::Text,
};
use embedded_hal::delay::DelayNs;
use embedded_hal_bus::i2c::RefCellDevice;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
    analog::adc::{Adc, AdcConfig, Attenuation},
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    i2c::master::{Config as I2cConfig, I2c},
    main,
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi},
    },
    time::Rate,
    uart::{Config as UartConfig, Uart},
};
use panic_rtt_target as _;

// Display Driver Logic
use mipidsi::{
    Builder,
    dcs::{Dcs, SetAddressMode},
    error::InitError,
    models::Model,
    options::{ColorInversion, ColorOrder, ModelOptions, Orientation, RefreshOrder, Rotation},
};

// Sensor Drivers
use bme280::i2c::BME280;
use pmsx003::PmsX003Sensor;

esp_bootloader_esp_idf::esp_app_desc!();

// --- ST7735R Custom Model ---

pub struct St7735r;

impl Model for St7735r {
    type ColorFormat = Rgb565;
    const FRAMEBUFFER_SIZE: (u16, u16) = (162, 132);

    fn init<RST, DELAY, DI>(
        &mut self,
        dcs: &mut Dcs<DI>,
        delay: &mut DELAY,
        options: &ModelOptions,
        rst: &mut Option<RST>,
    ) -> Result<SetAddressMode, InitError<RST::Error>>
    where
        RST: embedded_hal::digital::OutputPin,
        DELAY: DelayNs,
        DI: display_interface::WriteOnlyDataCommand,
    {
        match rst {
            Some(rst) => {
                rst.set_low().map_err(|_| InitError::DisplayError)?;
                delay.delay_ms(10);
                rst.set_high().map_err(|_| InitError::DisplayError)?;
                delay.delay_ms(150);
            }
            None => {
                dcs.write_command(mipidsi::dcs::SoftReset)
                    .map_err(|_| InitError::DisplayError)?;
                delay.delay_ms(150);
            }
        }

        dcs.write_command(mipidsi::dcs::ExitSleepMode)
            .map_err(|_| InitError::DisplayError)?;
        delay.delay_ms(500);

        // Power & Frame Rate
        dcs.write_raw(0xB1, &[0x01, 0x2C, 0x2D])
            .map_err(|_| InitError::DisplayError)?;
        dcs.write_raw(0xB2, &[0x01, 0x2C, 0x2D])
            .map_err(|_| InitError::DisplayError)?;
        dcs.write_raw(0xB3, &[0x01, 0x2C, 0x2D, 0x01, 0x2C, 0x2D])
            .map_err(|_| InitError::DisplayError)?;
        dcs.write_raw(0xB4, &[0x07])
            .map_err(|_| InitError::DisplayError)?;

        // Power Control
        dcs.write_raw(0xC0, &[0xA2, 0x02, 0x84])
            .map_err(|_| InitError::DisplayError)?;
        dcs.write_raw(0xC1, &[0xC5])
            .map_err(|_| InitError::DisplayError)?;
        dcs.write_raw(0xC2, &[0x0A, 0x00])
            .map_err(|_| InitError::DisplayError)?;
        dcs.write_raw(0xC3, &[0x8A, 0x2A])
            .map_err(|_| InitError::DisplayError)?;
        dcs.write_raw(0xC4, &[0x8A, 0xEE])
            .map_err(|_| InitError::DisplayError)?;
        dcs.write_raw(0xC5, &[0x0E])
            .map_err(|_| InitError::DisplayError)?;

        // Gamma
        dcs.write_raw(
            0xE0,
            &[
                0x0f, 0x1a, 0x0f, 0x18, 0x2f, 0x28, 0x20, 0x22, 0x1f, 0x1b, 0x23, 0x37, 0x00, 0x07,
                0x02, 0x10,
            ],
        )
        .map_err(|_| InitError::DisplayError)?;
        dcs.write_raw(
            0xE1,
            &[
                0x0f, 0x1b, 0x0f, 0x17, 0x33, 0x2c, 0x29, 0x2e, 0x30, 0x30, 0x39, 0x3f, 0x00, 0x07,
                0x03, 0x10,
            ],
        )
        .map_err(|_| InitError::DisplayError)?;

        dcs.write_raw(0x3A, &[0x05])
            .map_err(|_| InitError::DisplayError)?;
        dcs.write_raw(0x21, &[])
            .map_err(|_| InitError::DisplayError)?;

        dcs.write_command(mipidsi::dcs::EnterNormalMode)
            .map_err(|_| InitError::DisplayError)?;
        delay.delay_ms(10);
        dcs.write_command(mipidsi::dcs::SetDisplayOn)
            .map_err(|_| InitError::DisplayError)?;
        delay.delay_ms(100);

        Ok(SetAddressMode::new(
            options.color_order,
            options.orientation,
            RefreshOrder::default(),
        ))
    }

    fn write_pixels<DI, I>(
        &mut self,
        dcs: &mut Dcs<DI>,
        colors: I,
    ) -> Result<(), display_interface::DisplayError>
    where
        DI: display_interface::WriteOnlyDataCommand,
        I: IntoIterator<Item = Self::ColorFormat>,
    {
        dcs.write_command(mipidsi::dcs::WriteMemoryStart)
            .map_err(|_| display_interface::DisplayError::BusWriteError)?;
        for color in colors {
            let bytes = color.to_be_bytes();
            dcs.di
                .send_data(display_interface::DataFormat::U8(&bytes))?;
        }
        Ok(())
    }
}

// --- LTR559 Light Driver (Simplified) ---

const LTR559_ADDR: u8 = 0x23;

struct Ltr559<I2C> {
    i2c: I2C,
}

impl<I2C> Ltr559<I2C>
where
    I2C: embedded_hal::i2c::I2c,
{
    fn new(i2c: I2C, delay: &mut Delay) -> Result<Self, I2C::Error> {
        let mut sensor = Ltr559 { i2c };
        sensor.init(delay)?;
        Ok(sensor)
    }

    fn init(&mut self, delay: &mut Delay) -> Result<(), I2C::Error> {
        self.i2c.write(LTR559_ADDR, &[0x80, 0x02])?; // Soft Reset
        delay.delay_millis(100);
        self.i2c.write(LTR559_ADDR, &[0x80, 0x01 | (0x02 << 2)])?; // Active, Gain=4x
        self.i2c.write(LTR559_ADDR, &[0x85, 0x08])?; // 50ms / 50ms
        self.i2c.write(LTR559_ADDR, &[0x81, 0x03 | 0x20])?; // PS Active
        self.i2c.write(LTR559_ADDR, &[0x82, 0x13])?; // PS LED: 50mA
        self.i2c.write(LTR559_ADDR, &[0x83, 0x0A])?; // PS N Pulses: 10
        self.i2c.write(LTR559_ADDR, &[0x84, 0x02])?; // PS Meas Rate: 100ms
        Ok(())
    }

    fn read_als_lux(&mut self) -> Result<f32, I2C::Error> {
        let mut buffer = [0u8; 4];
        self.i2c.write_read(LTR559_ADDR, &[0x88], &mut buffer)?;
        let ch1 = u16::from_le_bytes([buffer[0], buffer[1]]) as f32;
        let ch0 = u16::from_le_bytes([buffer[2], buffer[3]]) as f32;

        let ratio = if (ch0 + ch1) > 0.0 {
            (ch1 * 100.0) / (ch1 + ch0)
        } else {
            101.0
        };
        let (c0, c1) = if ratio < 45.0 {
            (17743.0, -11059.0)
        } else if ratio < 64.0 {
            (42785.0, 19548.0)
        } else if ratio < 85.0 {
            (5926.0, -1185.0)
        } else {
            (0.0, 0.0)
        };
        let lux = ((ch0 * c0) - (ch1 * c1)) / 0.5 / 4.0 / 10000.0;
        Ok(lux.max(0.0))
    }

    fn read_ps(&mut self) -> Result<u16, I2C::Error> {
        let mut buffer = [0u8; 2];
        self.i2c.write_read(LTR559_ADDR, &[0x8D], &mut buffer)?;
        Ok(u16::from_le_bytes([buffer[0], buffer[1]]) & 0x07FF)
    }
}

struct SensorData {
    temp: f32,
    hum: f32,
    press: f32,
    lux: f32,
    prox: u16,
    r_ox: f32,
    r_red: f32,
    r_nh3: f32,
}

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut delay = Delay::new();

    info!("Initializing Enviro All Dashboard...");

    // 1. Power & Pins
    let _pwr = Output::new(peripherals.GPIO20, Level::High, OutputConfig::default());
    let mut m_en = Output::new(peripherals.GPIO3, Level::High, OutputConfig::default()); // heater EN
    m_en.set_high(); // Enable MICS6814 Heater
    delay.delay_millis(500);

    // 2. SPI Display Init
    let sck = peripherals.GPIO21;
    let mosi = peripherals.GPIO22;
    let dc = Output::new(peripherals.GPIO5, Level::Low, OutputConfig::default());
    let rst = Output::new(peripherals.GPIO7, Level::Low, OutputConfig::default());

    // --- SHARED PIN DEEP MAGIC ---
    // GPIO 6 (A2/D6) is physically shared between LCD_CS and the OX Gas Sensor.
    // We create two handles to the physical GPIO 6 to satisfy peripheral ownership:
    // 1. One handle goes to the Display's Digital Output (CS)
    // 2. One handle goes to the ADC's Analog Input (OX)
    let shared_handle_1 = unsafe { core::ptr::read(&peripherals.GPIO6) };
    let shared_handle_2 = peripherals.GPIO6;

    let cs = Output::new(shared_handle_1, Level::High, OutputConfig::default());
    // ----------------------------

    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(4))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sck)
    .with_mosi(mosi);
    let di = SPIInterface::new(ExclusiveDevice::new_no_delay(spi, cs).unwrap(), dc);

    let mut display = Builder::new(St7735r, di)
        .display_size(160, 80)
        .display_offset(1, 26)
        .invert_colors(ColorInversion::Normal)
        .color_order(ColorOrder::Bgr)
        .orientation(Orientation::default().rotate(Rotation::Deg0))
        .reset_pin(rst)
        .init(&mut delay)
        .unwrap();

    display.clear(Rgb565::BLACK).unwrap();

    // 3. I2C Bus Sharing
    let i2c_bus = I2c::new(peripherals.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(peripherals.GPIO19)
        .with_scl(peripherals.GPIO18);
    let i2c_ref_cell = RefCell::new(i2c_bus);

    let mut bme = BME280::new_primary(RefCellDevice::new(&i2c_ref_cell));
    bme.init(&mut delay).unwrap();

    let mut ltr = Ltr559::new(RefCellDevice::new(&i2c_ref_cell), &mut delay).unwrap();

    // 4. ADC (Gases)
    let mut adc_config = AdcConfig::new();
    let mut pin_nh3 = adc_config.enable_pin(peripherals.GPIO1, Attenuation::_11dB); // A0
    let mut pin_red = adc_config.enable_pin(peripherals.GPIO4, Attenuation::_11dB); // A1
    let mut pin_ox = adc_config.enable_pin(shared_handle_2, Attenuation::_11dB); // A2 (Shared GPIO 6)
    let mut adc = Adc::new(peripherals.ADC1, adc_config);

    // Warm up/Flush ADC channels for MICS sensors to help pull them off the saturation rail
    info!("Flushing ADC channels (50 samples)...");
    for _ in 0..50 {
        let _ = adc.read_oneshot(&mut pin_nh3);
        let _ = adc.read_oneshot(&mut pin_red);
        let _ = adc.read_oneshot(&mut pin_ox);
        delay.delay_millis(10);
    }

    // 5. UART (PMS5003) - Initialized inside the loop arm to allow buffer drainage
    let mut uart = Uart::new(peripherals.UART1, UartConfig::default().with_baudrate(9600))
        .unwrap()
        .with_tx(peripherals.GPIO16)
        .with_rx(peripherals.GPIO17);

    let label_style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
    let data_style = MonoTextStyle::new(&FONT_6X10, Rgb565::YELLOW);
    let title_style = MonoTextStyle::new(&FONT_6X10, Rgb565::CYAN);
    let outline_style = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb565::BLUE)
        .stroke_width(1)
        .build();

    let mut page = 0;

    info!("Dashboard started!");
    loop {
        // --- STEP 1: GLOBAL SENSING SWEEP (Top of every loop) ---

        // B. Climate (Fast) - Handle error without struct literal (private fields)
        let (temp, hum, press) = match bme.measure(&mut delay) {
            Ok(m) => (m.temperature, m.humidity, m.pressure / 100.0),
            Err(_) => (0.0, 0.0, 0.0),
        };

        // C. Light/Prox (Fast)
        let lux = ltr.read_als_lux().unwrap_or(0.0);
        let prox = ltr.read_ps().unwrap_or(0);

        // D. Gas (ADC Triple-sample)
        let _ = adc.read_oneshot(&mut pin_ox);
        delay.delay_millis(30);
        let _ = adc.read_oneshot(&mut pin_ox);
        delay.delay_millis(30);
        let raw_ox: u16 = adc.read_oneshot(&mut pin_ox).unwrap_or(0);

        let _ = adc.read_oneshot(&mut pin_red);
        delay.delay_millis(30);
        let _ = adc.read_oneshot(&mut pin_red);
        delay.delay_millis(30);
        let raw_red: u16 = adc.read_oneshot(&mut pin_red).unwrap_or(0);

        let _ = adc.read_oneshot(&mut pin_nh3);
        delay.delay_millis(30);
        let _ = adc.read_oneshot(&mut pin_nh3);
        delay.delay_millis(30);
        let raw_nh3: u16 = adc.read_oneshot(&mut pin_nh3).unwrap_or(0);

        fn calc_res(raw: u16) -> f32 {
            if raw == 0 {
                return 0.0;
            }
            if raw >= 4095 {
                return 100_000_000.0;
            }
            56000.0 / ((4095.0 / raw as f32) - 1.0)
        }

        let data = SensorData {
            temp,
            hum,
            press,
            lux,
            prox,
            r_ox: calc_res(raw_ox),
            r_red: calc_res(raw_red),
            r_nh3: calc_res(raw_nh3),
        };

        // --- STEP 2: DISPLAY RENDERING ---
        // ExclusiveDevice handles CS low/high during SPI transactions.
        display.clear(Rgb565::BLACK).unwrap();
        Rectangle::new(Point::new(0, 0), Size::new(160, 80))
            .into_styled(outline_style)
            .draw(&mut display)
            .unwrap();

        match page {
            0 => {
                // Climate Page
                Text::new("1/4 CLIMATE", Point::new(45, 12), title_style)
                    .draw(&mut display)
                    .unwrap();

                let mut buf = [0u8; 32];
                Text::new("Temp:", Point::new(5, 30), label_style)
                    .draw(&mut display)
                    .unwrap();
                let _ = core::fmt::write(
                    &mut format_wrapper::FormatWrapper(&mut buf),
                    format_args!("{:.2}", data.temp),
                );
                let val_str = core::str::from_utf8(&buf).unwrap().trim_matches('\0');
                Text::new(val_str, Point::new(45, 30), data_style)
                    .draw(&mut display)
                    .unwrap();
                Text::new(
                    "C",
                    Point::new(45 + (val_str.len() as i32 * 6) + 2, 30),
                    label_style,
                )
                .draw(&mut display)
                .unwrap();

                let mut buf = [0u8; 32];
                Text::new("Humid:", Point::new(5, 45), label_style)
                    .draw(&mut display)
                    .unwrap();
                let _ = core::fmt::write(
                    &mut format_wrapper::FormatWrapper(&mut buf),
                    format_args!("{:.2}", data.hum),
                );
                let val_str = core::str::from_utf8(&buf).unwrap().trim_matches('\0');
                Text::new(val_str, Point::new(45, 45), data_style)
                    .draw(&mut display)
                    .unwrap();
                Text::new(
                    "%",
                    Point::new(45 + (val_str.len() as i32 * 6) + 2, 45),
                    label_style,
                )
                .draw(&mut display)
                .unwrap();

                let mut buf = [0u8; 32];
                Text::new("Press:", Point::new(5, 60), label_style)
                    .draw(&mut display)
                    .unwrap();
                let _ = core::fmt::write(
                    &mut format_wrapper::FormatWrapper(&mut buf),
                    format_args!("{:.2}", data.press),
                );
                let val_str = core::str::from_utf8(&buf).unwrap().trim_matches('\0');
                Text::new(val_str, Point::new(45, 60), data_style)
                    .draw(&mut display)
                    .unwrap();
                Text::new(
                    "hPa",
                    Point::new(45 + (val_str.len() as i32 * 6) + 2, 60),
                    label_style,
                )
                .draw(&mut display)
                .unwrap();

                info!(
                    "Temperature: {} °C, Humidity: {} %, Pressure: {} hPa",
                    data.temp, data.hum, data.press
                );
            }
            1 => {
                // Light Page
                Text::new("2/4 LIGHT", Point::new(45, 12), title_style)
                    .draw(&mut display)
                    .unwrap();

                let mut buf = [0u8; 32];
                Text::new("Lux:", Point::new(5, 35), label_style)
                    .draw(&mut display)
                    .unwrap();
                let _ = core::fmt::write(
                    &mut format_wrapper::FormatWrapper(&mut buf),
                    format_args!("{:.2}", data.lux),
                );
                let val_str = core::str::from_utf8(&buf).unwrap().trim_matches('\0');
                Text::new(val_str, Point::new(45, 35), data_style)
                    .draw(&mut display)
                    .unwrap();
                Text::new(
                    "Lux",
                    Point::new(45 + (val_str.len() as i32 * 6) + 4, 35),
                    label_style,
                )
                .draw(&mut display)
                .unwrap();

                let mut buf = [0u8; 32];
                Text::new("Prox:", Point::new(5, 55), label_style)
                    .draw(&mut display)
                    .unwrap();
                let _ = core::fmt::write(
                    &mut format_wrapper::FormatWrapper(&mut buf),
                    format_args!("{}", data.prox),
                );
                Text::new(
                    core::str::from_utf8(&buf).unwrap().trim_matches('\0'),
                    Point::new(45, 55),
                    data_style,
                )
                .draw(&mut display)
                .unwrap();

                info!("Light: {} Lux, Proximity: {}", data.lux, data.prox);
            }
            2 => {
                // Gas Page - ADC Flush + Cached Display
                Text::new("3/4 GASES", Point::new(45, 12), title_style)
                    .draw(&mut display)
                    .unwrap();

                // Extra "Warm-up" sweep when on this page
                for _ in 0..30 {
                    let _ = adc.read_oneshot(&mut pin_ox);
                    let _ = adc.read_oneshot(&mut pin_red);
                    let _ = adc.read_oneshot(&mut pin_nh3);
                    delay.delay_millis(1);
                }

                let draw_ohm_local = |label: &str, y: i32, val: f32, display: &mut _| {
                    let mut buf = [0u8; 32];
                    Text::new(label, Point::new(5, y), label_style)
                        .draw(display)
                        .unwrap();
                    let unit: &str;
                    if val >= 1_000_000.0 {
                        let m = val / 1_000_000.0;
                        let _ = core::fmt::write(
                            &mut format_wrapper::FormatWrapper(&mut buf),
                            format_args!("{}.{}", m as u32, ((m % 1.0) * 10.0) as u32),
                        );
                        unit = "M-O";
                    } else if val >= 1_000.0 {
                        let k = val / 1_000.0;
                        let _ = core::fmt::write(
                            &mut format_wrapper::FormatWrapper(&mut buf),
                            format_args!("{}.{}", k as u32, ((k % 1.0) * 10.0) as u32),
                        );
                        unit = "K-O";
                    } else {
                        let _ = core::fmt::write(
                            &mut format_wrapper::FormatWrapper(&mut buf),
                            format_args!("{}", val as u32),
                        );
                        unit = "Ohms";
                    }
                    let val_str = core::str::from_utf8(&buf).unwrap().trim_matches('\0');
                    Text::new(val_str, Point::new(45, y), data_style)
                        .draw(display)
                        .unwrap();
                    Text::new(
                        unit,
                        Point::new(45 + (val_str.len() as i32 * 6) + 4, y),
                        label_style,
                    )
                    .draw(display)
                    .unwrap();
                };

                draw_ohm_local("OX:", 30, data.r_ox, &mut display);
                draw_ohm_local("RED:", 45, data.r_red, &mut display);
                draw_ohm_local("NH3:", 60, data.r_nh3, &mut display);

                info!("OX raw={} RED raw={} NH3 raw={}", raw_ox, raw_red, raw_nh3);
            }
            3 => {
                // Particle Page - Deep Sync with UI feedback
                Text::new("4/4 PARTICLES", Point::new(45, 12), title_style)
                    .draw(&mut display)
                    .unwrap();
                Text::new("SYNCING...", Point::new(45, 45), label_style)
                    .draw(&mut display)
                    .unwrap();

                info!("Syncing Particle sensor...");
                // 1. Drain stale backlog from MCU UART buffer to ensure we catch the LATEST frame
                while uart.read_ready() {
                    let mut discard = [0u8; 1];
                    let _ = uart.read(&mut discard);
                }

                let mut pms = PmsX003Sensor::new(&mut uart);
                let mut last_f = None;
                for _ in 0..20 {
                    match pms.read() {
                        Ok(f) => {
                            let is_nonzero = f.pm2_5 > 0 || f.pm1_0 > 0 || f.pm10 > 0;
                            last_f = Some(f);
                            if is_nonzero {
                                break;
                            }
                            delay.delay_millis(50);
                        }
                        Err(_) => {
                            delay.delay_millis(100);
                        }
                    }
                }

                if let Some(f) = last_f {
                    // Re-clear the data area to remove "SYNCING..." text
                    Rectangle::new(Point::new(1, 15), Size::new(158, 64))
                        .into_styled(
                            PrimitiveStyleBuilder::new()
                                .fill_color(Rgb565::BLACK)
                                .build(),
                        )
                        .draw(&mut display)
                        .unwrap();

                    info!(
                        "PM1.0: {} μg/m³, PM2.5: {} μg/m³, PM10: {} μg/m³",
                        f.pm1_0, f.pm2_5, f.pm10
                    );
                    let mut buf = [0u8; 32];
                    Text::new("PM1.0:", Point::new(5, 30), label_style)
                        .draw(&mut display)
                        .unwrap();
                    let _ = core::fmt::write(
                        &mut format_wrapper::FormatWrapper(&mut buf),
                        format_args!("{}", f.pm1_0),
                    );
                    let val_str = core::str::from_utf8(&buf).unwrap().trim_matches('\0');
                    Text::new(val_str, Point::new(45, 30), data_style)
                        .draw(&mut display)
                        .unwrap();
                    Text::new(
                        "ug",
                        Point::new(45 + (val_str.len() as i32 * 6) + 2, 30),
                        label_style,
                    )
                    .draw(&mut display)
                    .unwrap();

                    let mut buf = [0u8; 32];
                    Text::new("PM2.5:", Point::new(5, 45), label_style)
                        .draw(&mut display)
                        .unwrap();
                    let _ = core::fmt::write(
                        &mut format_wrapper::FormatWrapper(&mut buf),
                        format_args!("{}", f.pm2_5),
                    );
                    let val_str = core::str::from_utf8(&buf).unwrap().trim_matches('\0');
                    Text::new(val_str, Point::new(45, 45), data_style)
                        .draw(&mut display)
                        .unwrap();
                    Text::new(
                        "ug",
                        Point::new(45 + (val_str.len() as i32 * 6) + 2, 45),
                        label_style,
                    )
                    .draw(&mut display)
                    .unwrap();

                    let mut buf = [0u8; 32];
                    Text::new("PM10:", Point::new(5, 60), label_style)
                        .draw(&mut display)
                        .unwrap();
                    let _ = core::fmt::write(
                        &mut format_wrapper::FormatWrapper(&mut buf),
                        format_args!("{}", f.pm10),
                    );
                    let val_str = core::str::from_utf8(&buf).unwrap().trim_matches('\0');
                    Text::new(val_str, Point::new(45, 60), data_style)
                        .draw(&mut display)
                        .unwrap();
                    Text::new(
                        "ug",
                        Point::new(45 + (val_str.len() as i32 * 6) + 2, 60),
                        label_style,
                    )
                    .draw(&mut display)
                    .unwrap();
                }
            }
            _ => page = 0,
        }

        page = (page + 1) % 4;
        delay.delay_millis(1500); // 1.5-second instrument cycle
    }
}

mod format_wrapper {
    use core::fmt;

    pub struct FormatWrapper<'a>(pub &'a mut [u8]);

    impl<'a> fmt::Write for FormatWrapper<'a> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let bytes = s.as_bytes();
            let mut i = 0;
            while i < self.0.len() && i < bytes.len() {
                self.0[i] = bytes[i];
                i += 1;
            }
            Ok(())
        }
    }
}
