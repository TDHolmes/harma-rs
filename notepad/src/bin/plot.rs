//! Example that just prints all packets
use console::Term;
use heapless::spsc::Queue;
use rgb::RGB8;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};
use textplots::{Chart, ColorPlot, Shape};

use notepad::{
    comms,
    types::{self, imu},
};
use pensel_types::cli;

static mut A_QUEUE: Queue<imu::AccelerationVector, { types::ACC_QUEUE_SIZE }> = Queue::new();
static mut G_QUEUE: Queue<imu::GravityVector, { types::GRAV_QUEUE_SIZE }> = Queue::new();

const PURPLE: RGB8 = RGB8::new(0xE0, 0x80, 0xFF);
const RED: RGB8 = RGB8::new(0xFF, 0x00, 0x00);
const GREEN: RGB8 = RGB8::new(0x00, 0xFF, 0x00);

const PRINT_LEN: usize = 1000;

fn main() {
    let should_run = Arc::new(AtomicBool::new(true));
    let should_run_thread_ref = should_run.clone();
    let should_ctrlc_ref = should_run.clone();
    let mut serial = comms::PenselSerial::new_first_matching();

    // enable streaming, if it isn't already
    let enable_streaming_cmd = format!(
        "{} --{} --{}",
        cli::CMD_IMU,
        cli::ARG_ACCEL,
        cli::ARG_GRAVITY
    );
    serial.send_command(&enable_streaming_cmd).unwrap();

    let (a_producer, mut a_consumer) = unsafe { A_QUEUE.split() };
    let (g_producer, mut g_consumer) = unsafe { G_QUEUE.split() };

    let _sender = thread::spawn(move || {
        serial.parse_data_until(a_producer, g_producer, &should_run_thread_ref);
    });

    should_run.as_ref().store(true, Ordering::Release);

    let mut acc_x: [(f32, f32); PRINT_LEN] = [(0., 0.); PRINT_LEN];
    let mut acc_y: [(f32, f32); PRINT_LEN] = [(0., 0.); PRINT_LEN];
    let mut acc_z: [(f32, f32); PRINT_LEN] = [(0., 0.); PRINT_LEN];

    let mut grav_x: [(f32, f32); PRINT_LEN] = [(0., 0.); PRINT_LEN];
    let mut grav_y: [(f32, f32); PRINT_LEN] = [(0., 0.); PRINT_LEN];
    let mut grav_z: [(f32, f32); PRINT_LEN] = [(0., 0.); PRINT_LEN];

    let term = Term::stdout();
    term.hide_cursor().unwrap();

    ctrlc::set_handler(move || {
        let term = Term::stdout();
        term.show_cursor().unwrap();
        should_ctrlc_ref.as_ref().store(false, Ordering::Relaxed);
    })
    .unwrap();

    // let mut skip = 0;
    let mut g_update = false;
    let mut a_update = false;
    loop {
        if let Some(a) = a_consumer.dequeue() {
            acc_x.copy_within(1..PRINT_LEN, 0);
            acc_y.copy_within(1..PRINT_LEN, 0);
            acc_z.copy_within(1..PRINT_LEN, 0);
            acc_x[PRINT_LEN - 1] = (0., f32::from(a.x));
            acc_y[PRINT_LEN - 1] = (0., f32::from(a.y));
            acc_z[PRINT_LEN - 1] = (0., f32::from(a.z));
            for index in 0..PRINT_LEN {
                acc_x[index].0 += 1.;
                acc_y[index].0 += 1.;
                acc_z[index].0 += 1.;
            }
            a_update = true;
        }
        if let Some(g) = g_consumer.dequeue() {
            grav_x.copy_within(1..PRINT_LEN, 0);
            grav_y.copy_within(1..PRINT_LEN, 0);
            grav_z.copy_within(1..PRINT_LEN, 0);
            grav_x[PRINT_LEN - 1] = (0., f32::from(g.x));
            grav_y[PRINT_LEN - 1] = (0., f32::from(g.y));
            grav_z[PRINT_LEN - 1] = (0., f32::from(g.z));
            for index in 0..PRINT_LEN {
                grav_x[index].0 += 1.;
                grav_y[index].0 += 1.;
                grav_z[index].0 += 1.;
            }
            g_update = true;
        }

        if g_update && a_update {
            a_update = false;
            g_update = false;

            term.move_cursor_to(0, 0).unwrap();
            println!("Gravity Vector");
            Chart::new(200, 100, 0., 1000.)
                .linecolorplot(&Shape::Lines(&grav_x), RED)
                .linecolorplot(&Shape::Lines(&grav_y), GREEN)
                .linecolorplot(&Shape::Lines(&grav_z), PURPLE)
                .display();
            println!("\nLinear Acceleration");
            Chart::new(200, 100, 0., 1000.)
                .linecolorplot(&Shape::Lines(&acc_x), RED)
                .linecolorplot(&Shape::Lines(&acc_y), GREEN)
                .linecolorplot(&Shape::Lines(&acc_z), PURPLE)
                .display();
        }

        if !should_run.as_ref().load(Ordering::Acquire) {
            break;
        }
    }
}
