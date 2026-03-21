//! # Newxie Digital-to-Analog Thermometer Example for Adafruit Feather ESP32-C6
//!
//! It reads temperature and pressure from a BMP580 sensor and displays a
//! graphical thermometer and data on an Adafruit 1.14" 240x135 Color Newxie TFT Display.
//!
//! **Adafruit Products Used**:
//! - [Adafruit ESP32-C6 Feather](https://www.adafruit.com/product/5933)
//! - [Adafruit BMP580 Sensor (Stemma QT)](https://www.adafruit.com/product/6411)
//! - [Adafruit 1.14" 240x135 Color Newxie TFT Display](https://www.adafruit.com/product/4383)
//!
//! **Note**: As no mature crates for the BMP580 were available, this example
//! includes a minimal, self-contained blocking driver for the sensor.
//!
//! ## Hardware Overview
//! - **Display**: Adafruit 1.14" 240x135 Color Newxie TFT Display (using 135x240 Portrait mode).
//! - **Sensor**: BMP580 (connected via Stemma QT port).
//!
//! ## Wiring for Adafruit 1.14" Color Newxie TFT Breakout
//!
//! ```
//!      Breakout Pin  ->  Feather Label ->  ESP32-C6 GPIO
//! -----------------------------------------------------
//!      V+            ->  3.3V
//!      G             ->  GND
//!      CL (Clock)    ->  SCK           ->  IO21
//!      DA (Data)     ->  MO (MOSI)     ->  IO22
//!      CS (Chip Sel) ->  I07           ->  IO7
//!      DC (Data/Cmd) ->  A2/IO6        ->  IO6
//!      BL (B-Light)  ->  None          ->  Not Connected (Optional)
//! ```
//!
//! ## BMP580 Connection (Stemma QT)
//!
//! ```
//!      Sensor Pin    ->  Feather Label ->  ESP32-C6 GPIO
//! -----------------------------------------------------
//!      SCL           ->  SCL           ->  IO18
//!      SDA           ->  SDA           ->  IO19
//!      VIN           ->  3.3V
//!      GND           ->  GND
//!
//!      NOTE: GPIO 20 (Power Enable) must be HIGH to power the Stemma QT port.
//! ```
//!
//! Run with `cargo run --example newxie_thermometer`.

#![no_std]
#![no_main]

use defmt::info;
use display_interface_spi::SPIInterface;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_9X15_BOLD},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    i2c::master::{Config as I2cConfig, I2c},
    main,
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi},
    },
    time::Rate,
};
use mipidsi::{
    Builder,
    models::ST7789,
    options::{ColorOrder, Orientation, Rotation},
};
use panic_rtt_target as _;

esp_bootloader_esp_idf::esp_app_desc!();

// --- Minimal BMP580 Driver ---

struct Bmp580<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C> Bmp580<I2C>
where
    I2C: embedded_hal::i2c::I2c,
{
    pub fn new(i2c: I2C, address: u8) -> Self {
        Self { i2c, address }
    }

    pub fn init(&mut self) -> Result<(), I2C::Error> {
        let mut id = [0u8; 1];
        self.i2c.write_read(self.address, &[0x01], &mut id)?; // CHIP_ID
        info!("BMP580 Chip ID: 0x{:x}", id[0]);

        // Power mode: Normal (Normal mode = 0x01 in register 0x03)
        self.i2c.write(self.address, &[0x03, 0x01])?;

        // ODR: 50Hz (Register 0x37 bit 0x0F as per Arduino)
        self.i2c.write(self.address, &[0x37, 0x0F])?;

        Ok(())
    }

    pub fn read_data(&mut self) -> Result<(f32, f32), I2C::Error> {
        let mut data = [0u8; 6];
        self.i2c.write_read(self.address, &[0x1D], &mut data)?;

        let raw_t = ((data[2] as u32) << 16) | ((data[1] as u32) << 8) | (data[0] as u32);
        let raw_p = ((data[5] as u32) << 16) | ((data[4] as u32) << 8) | (data[3] as u32);

        let raw_t_signed = if raw_t & 0x800000 != 0 {
            (raw_t | 0xFF000000) as i32
        } else {
            raw_t as i32
        };

        let temp = raw_t_signed as f32 / 65536.0;
        let press = raw_p as f32 / 64.0;

        Ok((temp, press))
    }
}

// --- Thermometer Graphic ---

#[derive(Clone, Copy)]
struct ThermometerColors {
    bg: Rgb565,
    outline: Rgb565,
    bulb_outline: Rgb565,
    bulb_fill: Rgb565,
    tick_major: Rgb565,
    fill_actual: Rgb565,
}

impl Default for ThermometerColors {
    fn default() -> Self {
        Self {
            bg: Rgb565::BLACK,
            outline: Rgb565::WHITE,
            bulb_outline: Rgb565::WHITE,
            bulb_fill: Rgb565::RED,
            tick_major: Rgb565::WHITE,
            fill_actual: Rgb565::RED,
        }
    }
}

struct ThermometerGraphic {
    anchor: Point,
    width: u32,
    height: u32,
    temp_min: f32,
    temp_max: f32,
    colors: ThermometerColors,
}

impl ThermometerGraphic {
    fn new(anchor: Point, width: u32, height: u32, temp_min: f32, temp_max: f32) -> Self {
        Self {
            anchor,
            width,
            height,
            temp_min,
            temp_max,
            colors: ThermometerColors::default(),
        }
    }

    fn draw_static<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        Rectangle::new(self.anchor, Size::new(self.width, self.height))
            .into_styled(PrimitiveStyle::with_fill(self.colors.bg))
            .draw(target)?;

        let bulb_radius: i32 = 12;
        let bulb_center =
            self.anchor + Point::new(self.width as i32 / 2, self.height as i32 - bulb_radius - 10);

        Circle::with_center(bulb_center, (bulb_radius * 2) as u32)
            .into_styled(PrimitiveStyle::with_fill(self.colors.bulb_fill))
            .draw(target)?;

        Circle::with_center(bulb_center, (bulb_radius * 2) as u32)
            .into_styled(PrimitiveStyle::with_stroke(self.colors.bulb_outline, 1))
            .draw(target)?;

        let tube_width: u32 = 10;
        let tube_height = self.height - (bulb_radius as u32 * 2) - 20;
        let tube_top_left =
            self.anchor + Point::new((self.width as i32 / 2) - (tube_width as i32 / 2), 10);

        Rectangle::new(tube_top_left, Size::new(tube_width, tube_height))
            .into_styled(PrimitiveStyle::with_stroke(self.colors.outline, 1))
            .draw(target)?;

        let margin_top: i32 = 20;
        let margin_bottom: i32 = bulb_radius * 2 + 25;
        let active_height = self.height as i32 - margin_top - margin_bottom;
        let px_per_degree = active_height as f32 / (self.temp_max - self.temp_min);

        for temp in (self.temp_min as i32..=self.temp_max as i32).step_by(10) {
            let y = (self.anchor.y + margin_top + active_height)
                - ((temp as f32 - self.temp_min) * px_per_degree) as i32;
            Line::new(
                Point::new(tube_top_left.x - 5, y),
                Point::new(tube_top_left.x, y),
            )
            .into_styled(PrimitiveStyle::with_stroke(self.colors.tick_major, 1))
            .draw(target)?;
        }

        Ok(())
    }

    fn update_temp<D>(&self, target: &mut D, temp_f: f32) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let bulb_radius: i32 = 12;
        let margin_top: i32 = 20;
        let margin_bottom: i32 = bulb_radius * 2 + 25;
        let active_height = self.height as i32 - margin_top - margin_bottom;
        let px_per_degree = active_height as f32 / (self.temp_max - self.temp_min);

        let tube_width: i32 = 6;
        let tube_top_left =
            self.anchor + Point::new((self.width as i32 / 2) - (tube_width / 2), 10);

        let inner_tube_height = self.height - (bulb_radius as u32 * 2) - 22;
        Rectangle::new(
            tube_top_left + Point::new(1, 1),
            Size::new((tube_width - 2) as u32, inner_tube_height),
        )
        .into_styled(PrimitiveStyle::with_fill(self.colors.bg))
        .draw(target)?;

        let fill_height =
            ((temp_f - self.temp_min) * px_per_degree).clamp(0.0, active_height as f32);
        let fill_y = (self.anchor.y + margin_top + active_height) - fill_height as i32;

        Rectangle::new(
            Point::new(tube_top_left.x + 2, fill_y),
            Size::new((tube_width - 4) as u32, fill_height as u32),
        )
        .into_styled(PrimitiveStyle::with_fill(self.colors.fill_actual))
        .draw(target)?;

        Ok(())
    }
}

// --- Main ---

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut delay = Delay::new();

    info!("Initializing Newxie Thermometer...");

    // Note: GPIO 20 MUST be HIGH to power the Stemma QT port on the Feather ESP32-C6
    info!("Enabling Stemma/I2C Power (GPIO 20)");
    let _pwr = Output::new(peripherals.GPIO20, Level::High, OutputConfig::default());
    delay.delay_millis(500); // Wait for sensor to power up

    let i2c_config = I2cConfig::default().with_frequency(Rate::from_khz(100));
    let i2c = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(peripherals.GPIO19)
        .with_scl(peripherals.GPIO18);

    // Using 0x47 which was found in the scan
    let mut bmp = Bmp580::new(i2c, 0x47);
    if bmp.init().is_err() {
        info!("Failed to initialize BMP580 at 0x47");
    } else {
        info!("BMP580 initialized successfully at 0x47");
    }

    // 2. Initialize SPI for ST7789 Display
    info!("Initializing SPI Display...");
    let sck = peripherals.GPIO21;
    let mosi = peripherals.GPIO22;
    let miso = peripherals.GPIO23;
    let cs = Output::new(peripherals.GPIO7, Level::High, OutputConfig::default());
    let dc = Output::new(peripherals.GPIO6, Level::Low, OutputConfig::default());

    // RST (IO5) is NOT connected in this setup. mipidsi will use software reset.

    // Backlight (optional, setting high just in case)
    let _backlight = Output::new(peripherals.GPIO4, Level::High, OutputConfig::default());

    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(40))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sck)
    .with_mosi(mosi)
    .with_miso(miso);

    let spi_device = ExclusiveDevice::new(spi, cs, delay).unwrap();
    let di = SPIInterface::new(spi_device, dc);

    // Final settings for Adafruit 1.14" TFT
    info!("Building Display Driver (135x240 Portrait)...");
    let mut display = Builder::new(ST7789, di)
        .display_size(135, 240)
        .display_offset(52, 40)
        .invert_colors(mipidsi::options::ColorInversion::Inverted)
        .orientation(Orientation {
            rotation: Rotation::Deg0,
            mirrored: false,
        })
        .color_order(ColorOrder::Rgb)
        .init(&mut delay)
        .unwrap();

    info!("Display Init Complete. Clearing to Black...");
    display.clear(Rgb565::BLACK).unwrap();

    // 3. Setup Thermometer Graphic (Fits nicely in 135x240 Portrait)
    let therm = ThermometerGraphic::new(Point::new(10, 5), 115, 180, 40.0, 100.0);
    therm.draw_static(&mut display).unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_9X15_BOLD)
        .text_color(Rgb565::WHITE)
        .build();

    info!("Starting main loop...");

    let mut cycle: i32 = 0;
    let mut last_temp_c: f32 = 22.0;
    let mut last_press_hpa: f32 = 1013.25;

    loop {
        if let Ok((temp, press)) = bmp.read_data() {
            last_temp_c = temp;
            last_press_hpa = press / 100.0;
        }
        // Log temperature and pressure with consistent precision (1 decimal place)
        let mut log_buf = [0u8; 64];
        let _ = core::fmt::write(
            &mut Writer(&mut log_buf),
            format_args!(
                "Temp: {:.1} C | Pressure: {:.1} hPa",
                last_temp_c, last_press_hpa
            ),
        );
        info!(
            "{}",
            core::str::from_utf8(&log_buf).unwrap().trim_matches('\0')
        );

        let temp_f = (last_temp_c * 9.0 / 5.0) + 32.0;
        therm.update_temp(&mut display, temp_f).unwrap();

        cycle = (cycle + 1) % 4;

        // Clear label area at the bottom
        Rectangle::new(Point::new(0, 190), Size::new(135, 50))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(&mut display)
            .unwrap();

        let mut buf = [0u8; 32];
        let text = match cycle {
            0 => {
                let _ = core::fmt::write(&mut Writer(&mut buf), format_args!("{:.1} F", temp_f));
                core::str::from_utf8(&buf).unwrap().trim_matches('\0')
            }
            1 => {
                let _ =
                    core::fmt::write(&mut Writer(&mut buf), format_args!("{:.1} C", last_temp_c));
                core::str::from_utf8(&buf).unwrap().trim_matches('\0')
            }
            2 => {
                let _ = core::fmt::write(
                    &mut Writer(&mut buf),
                    format_args!("{:.1} hPa", last_press_hpa),
                );
                core::str::from_utf8(&buf).unwrap().trim_matches('\0')
            }
            _ => "Newxie",
        };

        // Center text at the bottom
        Text::with_baseline(text, Point::new(10, 200), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();

        delay.delay_millis(2000);
    }
}

struct Writer<'a>(&'a mut [u8]);
impl<'a> core::fmt::Write for Writer<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let len = s.len();
        if len > self.0.len() {
            return Err(core::fmt::Error);
        }
        let (head, tail) = core::mem::take(&mut self.0).split_at_mut(len);
        head.copy_from_slice(s.as_bytes());
        self.0 = tail;
        Ok(())
    }
}
