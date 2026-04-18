//! This example drives the 0.96" 160x80 ST7735R display on the Enviro+ FeatherWing.
//!
//! ## Why a Custom ST7735R Driver?
//! We implement a custom `Model` because the standard `mipidsi` ST7735S driver enforces
//! strict coordinate validation. Applying the required (1, 26) offsets for this 0.96" panel
//! would cause the library to panic (assertion failure: width + offset_x <= max_width).
//! Our custom `St7735r` model bypasses this by reporting a logical framebuffer of 162x132
//! to provide the necessary headroom for the centered offsets.
//!
//! ## Technical Rationale: The (1, 26) Offsets
//! The ST7735 controller memory is 132x162, but the physical glass is only 160x80.
//! To center the image, we apply offsets based on the controller's native Portrait orientation:
//! - **Vertical (26)**: (132 native width - 80 glass height) / 2 = 26 pixels.
//! - **Horizontal (1)**: (162 native height - 160 glass width) / 2 = 1 pixel.
//!
//! ## Official Ecosystem References
//! - **Pimoroni**: Their CircuitPython driver defines these as `colstart=26` and `rowstart=1` for the 0.96" model.
//! - **Adafruit**: Their ST7735 Arduino library refers to this as the **"Green Tab"** or **"Mini TFT"** configuration.
//!
//! ## Verifiable Sources
//! - **Pimoroni Reference**: https://github.com/pimoroni/st7735-python/blob/master/library/ST7735/__init__.py
//! - **Adafruit Reference**: https://github.com/adafruit/Adafruit-ST7735-Library
//!
//! ## Pin Mapping (Feather C6 Silk Labels)
//! - **SCK**: GPIO 21
//! - **MOSI**: GPIO 22
//! - **LCD_DC (Slot D5)**:  GPIO 5
//! - **LCD_CS (Slot D6)**:  GPIO 6
//! - **LCD_RST & BL (Slot D9)**: GPIO 7
//! - **Board Power Control**: GPIO 20
//!
//! Run with `cargo run --example enviro_st7735r_spi`.

#![no_std]
#![no_main]

use defmt::info;
use display_interface::{DataFormat, WriteOnlyDataCommand};
use display_interface_spi::SPIInterface;
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_7X13},
    pixelcolor::{Rgb565, RgbColor},
    prelude::*,
    primitives::{Circle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment},
    text::Text,
};
use embedded_hal::{delay::DelayNs, digital::OutputPin};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    main,
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi},
    },
    time::Rate,
};
use mipidsi::{
    Builder,
    dcs::{Dcs, SetAddressMode},
    error::{Error, InitError},
    models::*,
    options::{ColorInversion, ColorOrder, ModelOptions, Orientation, RefreshOrder, Rotation},
};

/// Custom Model for ST7735R based on Adafruit's initialization sequence
///
/// This custom model is required because the standard `mipidsi` ST7735S model enforces
/// strict coordinate validation that causes runtime panics when applying the required
/// (1, 26) glass offsets. By reporting a logical buffer of 162x132, we provide
/// the necessary headroom for the centered landscape window.
pub struct St7735r;

impl Model for St7735r {
    type ColorFormat = Rgb565;
    // The ST7735R controller has a 132x162 pixel RAM.
    //
    // RATIONALE FOR (162, 132):
    // The mipidsi 0.8.0 Builder performs a strict check: `width + offset_x <= max_width`.
    // For a 160x80 landscape request with a 1px offset, this results in `160 + 1 = 161`.
    // If we define the native width as 132 (the physical chip limit), the library panics.
    // By logically swapping the limits to (162, 132), we move the 'long' 160-pixel
    // axis into the 162-pixel headroom, allowing landscape requests to pass validation.
    const FRAMEBUFFER_SIZE: (u16, u16) = (162, 132);

    fn init<RST, DELAY, DI>(
        &mut self,
        dcs: &mut Dcs<DI>,
        delay: &mut DELAY,
        options: &ModelOptions,
        rst: &mut Option<RST>,
    ) -> Result<SetAddressMode, InitError<RST::Error>>
    where
        RST: OutputPin,
        DELAY: DelayNs,
        DI: WriteOnlyDataCommand,
    {
        // 1. Hardware Reset (Adafruit Standard)
        match rst {
            Some(rst) => self.hard_reset(rst, delay)?,
            None => {
                dcs.write_command(mipidsi::dcs::SoftReset)
                    .map_err(|_| InitError::DisplayError)?;
                delay.delay_ms(150);
            }
        }

        // 2. Exit Sleep
        dcs.write_command(mipidsi::dcs::ExitSleepMode)
            .map_err(|_| InitError::DisplayError)?;
        delay.delay_ms(500);

        // 3. Power & Frame Rate Control (Reference: Pimoroni screen.py / Adafruit_ST7735.cpp)
        // These specific values stabilize the 0.96" panel's charge pumps and oscillator.
        dcs.write_raw(0xB1, &[0x01, 0x2C, 0x2D])
            .map_err(|_| InitError::DisplayError)?; // FRMCTR1
        dcs.write_raw(0xB2, &[0x01, 0x2C, 0x2D])
            .map_err(|_| InitError::DisplayError)?; // FRMCTR2
        dcs.write_raw(0xB3, &[0x01, 0x2C, 0x2D, 0x01, 0x2C, 0x2D])
            .map_err(|_| InitError::DisplayError)?; // FRMCTR3

        // 4. Column Inversion (Required for the 0.96" 160x80 wide-angle display)
        dcs.write_raw(0xB4, &[0x07])
            .map_err(|_| InitError::DisplayError)?; // INVCTR

        // 5. Power Control (Source: Adafruit InitR_MiniTFT FB set)
        dcs.write_raw(0xC0, &[0xA2, 0x02, 0x84])
            .map_err(|_| InitError::DisplayError)?; // PWCTR1
        dcs.write_raw(0xC1, &[0xC5])
            .map_err(|_| InitError::DisplayError)?; // PWCTR2
        dcs.write_raw(0xC2, &[0x0A, 0x00])
            .map_err(|_| InitError::DisplayError)?; // PWCTR3
        dcs.write_raw(0xC3, &[0x8A, 0x2A])
            .map_err(|_| InitError::DisplayError)?; // PWCTR4
        dcs.write_raw(0xC4, &[0x8A, 0xEE])
            .map_err(|_| InitError::DisplayError)?; // PWCTR5
        dcs.write_raw(0xC5, &[0x0E])
            .map_err(|_| InitError::DisplayError)?; // VMCTR1

        // 6. Gamma Correction (Source: Verified Pimoroni 0.96" Look-up Table)
        dcs.write_raw(
            0xE0,
            &[
                0x02, 0x1C, 0x07, 0x12, 0x37, 0x32, 0x29, 0x2D, 0x29, 0x25, 0x2B, 0x39, 0x00, 0x01,
                0x03, 0x10,
            ],
        )
        .map_err(|_| InitError::DisplayError)?; // GMCTRP1
        dcs.write_raw(
            0xE1,
            &[
                0x03, 0x1D, 0x07, 0x06, 0x2E, 0x2C, 0x29, 0x2D, 0x2E, 0x2E, 0x37, 0x3F, 0x00, 0x00,
                0x02, 0x10,
            ],
        )
        .map_err(|_| InitError::DisplayError)?; // GMCTRN1

        // 7. Component-specific overrides
        // 0x3A = COLMOD (16-bit color)
        // 0x21 = INVON (Important: This panel is physically inverted)
        dcs.write_raw(0x3A, &[0x05])
            .map_err(|_| InitError::DisplayError)?;
        dcs.write_raw(0x21, &[])
            .map_err(|_| InitError::DisplayError)?;

        // 8. Finalize Normal Mode
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

    fn write_pixels<DI, I>(&mut self, dcs: &mut Dcs<DI>, colors: I) -> Result<(), Error>
    where
        DI: WriteOnlyDataCommand,
        I: IntoIterator<Item = Self::ColorFormat>,
    {
        dcs.write_command(mipidsi::dcs::WriteMemoryStart)?;
        let mut iter = colors.into_iter().map(|c| c.into_storage());
        dcs.di.send_data(DataFormat::U16BEIter(&mut iter))
    }
}
use panic_rtt_target as _;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut delay = Delay::new();

    info!("Initializing Enviro+ Display (Stable Mapping)...");

    // 1. Enable Hardware Power (GPIO 20)
    let _pwr20 = Output::new(peripherals.GPIO20, Level::High, OutputConfig::default());
    delay.delay_millis(1000); // 1s wait for full bus stability

    // 3. Setup Status LED (GPIO 15)
    let mut status_led = Output::new(peripherals.GPIO15, Level::Low, OutputConfig::default());
    status_led.set_high();

    // 3. Configure Communications (Native C6 SPI)
    let sck = peripherals.GPIO21;
    let mosi = peripherals.GPIO22;
    let miso = peripherals.GPIO23;
    let dc = Output::new(peripherals.GPIO5, Level::Low, OutputConfig::default()); // Slot D5
    let cs = Output::new(peripherals.GPIO6, Level::High, OutputConfig::default()); // Slot D6
    let rst = Output::new(peripherals.GPIO7, Level::High, OutputConfig::default()); // Slot D9 (Backlight + Reset)

    // 4. Setup Control Signals (Slot D5, D6, D9)
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(4))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sck)
    .with_mosi(mosi)
    .with_miso(miso);

    let spi_device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    let di = SPIInterface::new(spi_device, dc);

    // 5. Initialize Display (Stable Centered Landscape)
    //
    // Rationale:
    // - Rotation::Deg0 is the base landscape axis for the Enviro+ glass.
    // - Offset (1, 26) centers the 160x80 image perfectly within the physical glass area,
    //   while avoiding edge-wrapping ghost lines seen at larger offsets.
    let mut display = Builder::new(St7735r, di)
        .display_size(160, 80) // Full Screen Landscape
        .display_offset(1, 26) // Stable physical center
        .invert_colors(ColorInversion::Normal)
        .color_order(ColorOrder::Bgr)
        .orientation(Orientation::default().rotate(Rotation::Deg0))
        .reset_pin(rst)
        .init(&mut delay)
        .unwrap();

    status_led.set_low();
    info!("Display initialized successfully!");

    // 6. Bootstrap: Initial Color Cycle
    for color in [Rgb565::RED, Rgb565::GREEN, Rgb565::BLUE] {
        status_led.toggle();
        display.clear(color).unwrap();
        delay.delay_millis(1000);
    }

    // 7. Transition to Gallery (Landscape Dashboard)
    display.clear(Rgb565::BLACK).unwrap();

    // 7.1 Draw Title (White)
    let character_style = MonoTextStyle::new(&FONT_7X13, Rgb565::WHITE);
    Text::new("FEATHER C6 ENVIRO+", Point::new(15, 25), character_style)
        .draw(&mut display)
        .unwrap();

    // 7.2 Draw Primary Shape (Dashboard Frame - Blue)
    let style = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb565::BLUE)
        .stroke_width(2)
        .stroke_alignment(StrokeAlignment::Inside)
        .build();

    Rectangle::new(Point::new(5, 30), Size::new(150, 45))
        .into_styled(style)
        .draw(&mut display)
        .unwrap();

    // 7.3 Draw Status Indicator (Magenta)
    let circle_style = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::MAGENTA)
        .build();

    Circle::new(Point::new(130, 5), 15)
        .into_styled(circle_style)
        .draw(&mut display)
        .unwrap();

    loop {
        status_led.toggle();
        delay.delay_millis(1000);
    }
}
