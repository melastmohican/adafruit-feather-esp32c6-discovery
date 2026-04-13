# Design: Fix Enviro+ FeatherWing Examples for Adafruit Feather ESP32-C6

Update the `enviro_*` examples to work with the Adafruit Feather ESP32-C6 and the Pimoroni Enviro+ FeatherWing.

## 1. Research Findings

### 1.1 Hardware Context
- **Board:** Adafruit Feather ESP32-C6
- **Peripheral:** Pimoroni Enviro+ FeatherWing
- **Current State:** Examples are hardcoded for `ESP32-C3-DevKit-RUST-1`, which has different GPIO mappings and lacks the GPIO 20 power management requirement of the Feather C6.

### 1.2 Pin Mapping (Feather C6)

| Function | Enviro+ Label | Feather C6 GPIO | Conflict / Note |
| :--- | :--- | :--- | :--- |
| **I2C SDA** | SDA | **18** | Shared with Stemma QT |
| **I2C SCL** | SCL | **19** | Shared with Stemma QT |
| **I2C Power** | N/A | **20** | **MUST BE HIGH** to power Stemma/Headers |
| **SPI SCK** | SCK | **21** | |
| **SPI MOSI** | MO | **22** | |
| **LCD CS** | D6 | **6** | Shared with A2 (OX Gas Sensor) |
| **LCD DC** | D5 | **5** | Shared with A3 (Analog Input) |
| **MICS EN** | D4 | **4** | Heater enable |
| **NH3 Gas** | A0 | **0** | |
| **RED Gas** | A1 | **1** | |
| **OX Gas** | A2 | **6** | Shared with D6 (LCD CS) |
| **LCD BL/RST**| D9 | **9** | Shared with NeoPixel/Boot |

## 2. Approach

### 2.1 I2C Sensors (`enviro_bme280_i2c.rs`, `enviro_ltr559_i2c.rs`)
- Update SDA/SCL to 18/19.
- Initialize GPIO 20 as `Output` and set `High` early in `main`.
- Add 50ms delay after power-on.
- Remove references to ESP32-C3 in comments and headers.

### 2.2 Analog Gas Sensors (`enviro_mics6814.rs`)
- Update NH3, RED, OX pins to 0, 1, and 6.
- Update Heater Enable pin to 4.
- Add GPIO 20 power enable (just in case it affects any header rail, though usually only Stemma).
- Add explicit warning about the GPIO 6 conflict with the LCD.

### 2.3 SPI Display (`enviro_display_spi.rs`)
- Update SCK/MOSI/CS/DC/RST to 21/22/6/5/9.
- Remove "USB Loss" warnings as GPIO 5/6 are not USB pins on the C6.
- Add note about GPIO 9 backlight sharing with the onboard NeoPixel.
- Keep the `mipidsi` 0.8.0 configuration.

## 3. Implementation Details

### 3.1 Documentation
- Update file headers to specify "Adafruit Feather ESP32-C6".
- Update wiring diagrams in comments to match the C6 pinout.

### 3.2 Code Structure
- Ensure `esp_hal` 1.0.0 idiomatic usage (e.g., `esp_hal::init`, `Output::new`).
- Use `rtt_target` for logging as per project standards.

## 4. Testing & Validation

1. **Static Analysis:**
   - Verify all GPIO constants match the Feather C6 schematic.
   - Ensure `Cargo.toml` dependencies are sufficient.

2. **Functional Verification (Post-Implementation):**
   - Each example should compile for the `riscv32imac-unknown-none-elf` target.
   - `cargo run --example <name>` should successfully initialize hardware (mocked/verified via code review of pin logic).

## 5. Risks & Mitigations

- **Conflict:** GPIO 6 is shared between LCD CS and OX Gas.
  - *Mitigation:* Document this clearly. Advise users not to use both simultaneously in the same firmware without managing the pin state.
- **NeoPixel:** GPIO 9 backlight might be bright.
  - *Mitigation:* Note this side effect in the example description.
