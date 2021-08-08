#![no_std]
#![no_main]

use pensel::{bsp, hal, pac, serial_write, usb_serial::UsbSerial};

use cortex_m::{asm::wfi, peripheral::NVIC};
use panic_halt as _;

use bsp::entry;
use hal::{clock::GenericClockController, delay::Delay, prelude::*};
use pac::{interrupt, CorePeripherals, Peripherals};

static mut USB_SERIAL: Option<UsbSerial> = None;

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
    let mut red_led: bsp::RedLed = pins.d13.into();
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

    // initialize the IMU
    let mut imu = bno055::Bno055::new(i2c).with_alternative_address();
    if let Err(err) = imu.init(&mut delay) {
        handle_bno_err(&err, &mut delay);
    }
    imu.set_mode(bno055::BNO055OperationMode::NDOF, &mut delay)
        .unwrap();

    // perpetually read out angle data
    loop {
        let angles_res = imu.gravity();
        if let Ok(angles) = angles_res {
            serial_write!(
                USB_SERIAL,
                "{}, {}, {}\r\n",
                (angles.x * 1000.) as isize,
                (angles.y * 1000.) as isize,
                (angles.z * 1000.) as isize
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
        USB_SERIAL.as_mut().map(|serial| {
            let bytes_read = serial.poll(&mut buf);
            serial.write(&buf[0..bytes_read]);
        });
    }
}
