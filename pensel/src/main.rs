#![no_std]
#![no_main]

use pensel::{bsp, hal, pac, serial_write, usb_serial::UsbSerial};

use cortex_m::peripheral::NVIC;
use panic_persist as _;

use bsp::entry;
use hal::{clock::GenericClockController, delay::Delay, prelude::*};
use pac::{interrupt, CorePeripherals, Peripherals};

static mut USB_SERIAL: Option<UsbSerial> = None;
static SHOULD_START: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(true);

const BNO055_CALIBRATION: bno055::BNO055Calibration = bno055::BNO055Calibration {
    acc_offset_x_lsb: 2,
    acc_offset_x_msb: 0,
    acc_offset_y_lsb: 252,
    acc_offset_y_msb: 255,
    acc_offset_z_lsb: 231,
    acc_offset_z_msb: 255,
    mag_offset_x_lsb: 215,
    mag_offset_x_msb: 254,
    mag_offset_y_lsb: 174,
    mag_offset_y_msb: 1,
    mag_offset_z_lsb: 228,
    mag_offset_z_msb: 1,
    gyr_offset_x_lsb: 1,
    gyr_offset_x_msb: 0,
    gyr_offset_y_lsb: 0,
    gyr_offset_y_msb: 0,
    gyr_offset_z_lsb: 0,
    gyr_offset_z_msb: 0,
    acc_radius_lsb: 232,
    acc_radius_msb: 3,
    mag_radius_lsb: 241,
    mag_radius_msb: 2,
};

#[entry]
fn main() -> ! {
    // initialize core peripherals
    let mut peripherals = Peripherals::take().unwrap();
    let mut core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );
    let mut delay = Delay::new(core.SYST, &mut clocks);

    // initialize GPIOs
    let pins = bsp::Pins::new(peripherals.PORT);
    let mut _red_led: bsp::RedLed = pins.d13.into();
    let i2c = bsp::i2c_master(
        &mut clocks,
        400.khz(),
        peripherals.SERCOM3,
        &mut peripherals.PM,
        pins.sda,
        pins.scl,
    );

    // initialize USB
    unsafe {
        USB_SERIAL = Some(UsbSerial::new(
            peripherals.USB,
            &mut clocks,
            &mut peripherals.PM,
            pins.usb_dm,
            pins.usb_dp,
        ));
        core.NVIC.set_priority(interrupt::USB, 1);
        NVIC::unmask(interrupt::USB);
    }

    while SHOULD_START.load(core::sync::atomic::Ordering::Acquire) == false {
        cortex_m::asm::wfi();
    }

    // Check if there was a panic message, if so, send to UART
    if let Some(msg) = panic_persist::get_panic_message_bytes() {
        unsafe {
            USB_SERIAL.as_mut().unwrap().write(msg);
        }
    }

    // initialize the IMU
    let mut imu = bno055::Bno055::new(i2c).with_alternative_address();
    if let Err(err) = imu.init(&mut delay) {
        handle_bno_err(&err, &mut delay);
    }
    imu.set_mode(bno055::BNO055OperationMode::NDOF, &mut delay)
        .unwrap();

    imu.set_calibration_profile(BNO055_CALIBRATION, &mut delay)
        .unwrap();

    // perpetually read out angle data
    loop {
        // Get gravity vector
        let angles_res = imu.gravity_fixed();
        if let Ok(angles) = angles_res {
            serial_write!(
                USB_SERIAL,
                "G:{},{},{}\n",
                (angles.x as isize * 10),
                (angles.y as isize * 10),
                (angles.z as isize * 10)
            );
        }

        // get acceleration
        let lin_accel = imu.linear_acceleration_fixed();
        if let Ok(acc) = lin_accel {
            serial_write!(
                USB_SERIAL,
                "A:{},{},{}\n",
                (acc.x as isize * 10),
                (acc.y as isize * 10),
                (acc.z as isize * 10)
            );
        }
    }
}

fn handle_bno_err(error: &bno055::Error<hal::sercom::v1::I2CError>, delay: &mut Delay) -> ! {
    loop {
        delay.delay_ms(500_u32);
        serial_write!(USB_SERIAL, "imu err: ");
        match error {
            bno055::Error::I2c(hal::sercom::v1::I2CError::Nack) => {
                serial_write!(USB_SERIAL, "I2c nak\r\n")
            }
            bno055::Error::I2c(_) => serial_write!(USB_SERIAL, "I2c\r\n"),
            bno055::Error::InvalidChipId(_) => serial_write!(USB_SERIAL, "InvalidChipId\r\n"),
            bno055::Error::InvalidMode => serial_write!(USB_SERIAL, "InvalidMode\r\n"),
        };
    }
}

#[interrupt]
fn USB() {
    let mut buf = [0u8; 64];
    unsafe {
        if let Some(serial) = USB_SERIAL.as_mut() {
            let bytes_read = serial.poll(&mut buf);
            serial.write(&buf[0..bytes_read]);
            if bytes_read != 0 {
                SHOULD_START.store(true, core::sync::atomic::Ordering::Release);
            }
        }
    }
}
