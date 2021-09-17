//! Example that just prints all packets
use heapless::spsc::Queue;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use notepad::{
    comms,
    types::{self, imu},
};
use pensel_types::cli;

static mut A_QUEUE: Queue<imu::AccelerationVector, { types::ACC_QUEUE_SIZE }> = Queue::new();
static mut G_QUEUE: Queue<imu::GravityVector, { types::GRAV_QUEUE_SIZE }> = Queue::new();

fn main() {
    let should_run = Arc::new(AtomicBool::new(true));
    let should_run_thread_ref = should_run.clone();
    let should_run_ctrl_c = should_run.clone();
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

    // Parse & stream data until we receive a keyboard interrupt
    ctrlc::set_handler(move || {
        should_run_ctrl_c.as_ref().store(false, Ordering::Release);
    })
    .unwrap();
    let _sender = thread::spawn(move || {
        serial.parse_data_until(a_producer, g_producer, should_run_thread_ref)
    });

    // print until we're told to stop
    while should_run.as_ref().load(Ordering::Acquire) {
        if let Some(a) = a_consumer.dequeue() {
            println!("A: {:?}", a);
        }
        if let Some(g) = g_consumer.dequeue() {
            println!("G: {:?}", g);
        }
    }
}
