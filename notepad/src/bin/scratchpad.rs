//! Example that just prints all packets
use clap::{App, Arg};
use heapless::spsc::Queue;

use std::{
    fs::File,
    io::prelude::*,
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

enum Mode {
    Print,
    Record,
}

fn main() {
    let mut mode = Mode::Record;

    let matches = App::new("Scratchpad")
        .arg(
            Arg::new("record")
                .short('r')
                .long("record")
                .value_name("FILE")
                .help("Configures for recording to the given file")
                .takes_value(true),
        )
        .arg(
            Arg::new("print")
                .long("print")
                .help("just prints out accel/gravity packets"),
        )
        .arg(
            Arg::new("v")
                .short('v')
                .multiple_occurrences(true)
                .help("Sets the level of verbosity"),
        )
        .get_matches();

    if matches.is_present("print") {
        mode = Mode::Print;
    }

    let level = match matches.occurrences_of("v") {
        0 => log::Level::Warn,
        1 => log::Level::Info,
        2 => log::Level::Debug,
        _ => log::Level::Trace,
    };
    simple_logger::init_with_level(level).unwrap();

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
        serial.parse_data_until(a_producer, g_producer, &should_run_thread_ref);
    });

    // Do the action until we're told to stop
    match mode {
        Mode::Record => {
            println!("recording...");
            let filepath = matches.value_of("record").unwrap();
            let mut file = File::create(filepath).unwrap();
            while should_run.as_ref().load(Ordering::Acquire) {
                if let Some(a) = a_consumer.dequeue() {
                    writeln!(file, "{}", a).unwrap();
                }
                if let Some(g) = g_consumer.dequeue() {
                    writeln!(file, "{}", g).unwrap();
                }
            }
        }

        Mode::Print => {
            println!("printing...");
            while should_run.as_ref().load(Ordering::Acquire) {
                if let Some(a) = a_consumer.dequeue() {
                    println!("{}", a);
                }
                if let Some(g) = g_consumer.dequeue() {
                    println!("{}", g);
                }
            }
        }
    }

    println!("done!");
}
