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
    types::{self, AccelerationVec, GravityVec},
};

static mut A_QUEUE: Queue<AccelerationVec, { types::ACC_QUEUE_SIZE }> = Queue::new();
static mut G_QUEUE: Queue<GravityVec, { types::GRAV_QUEUE_SIZE }> = Queue::new();

const PRINT_LEN: usize = 1000;

fn main() {
    let should_run = Arc::new(AtomicBool::new(true));
    let should_run_thread_ref = should_run.clone();
    let should_ctrlc_ref = should_run.clone();
    let mut serial = comms::PenselSerial::new_first_matching();

    let (a_producer, mut a_consumer) = unsafe { A_QUEUE.split() };
    let (g_producer, mut g_consumer) = unsafe { G_QUEUE.split() };

    let _sender = thread::spawn(move || {
        serial.parse_data_until(a_producer, g_producer, should_run_thread_ref)
    });

    should_run.as_ref().store(true, Ordering::Release);

    let mut x_points: [(f32, f32); PRINT_LEN] = [(0., 0.); PRINT_LEN];
    let mut y_points: [(f32, f32); PRINT_LEN] = [(0., 0.); PRINT_LEN];
    let mut z_points: [(f32, f32); PRINT_LEN] = [(0., 0.); PRINT_LEN];

    println!("\ny = interpolated points");
    let term = Term::stdout();
    term.hide_cursor().unwrap();

    ctrlc::set_handler(move || {
        let term = Term::stdout();
        term.show_cursor().unwrap();
        should_ctrlc_ref.as_ref().store(false, Ordering::Relaxed);
    })
    .unwrap();

    // let mut skip = 0;
    loop {
        if let Some(_a) = a_consumer.dequeue() {
            // println!("A: {:?}", a);
        }
        if let Some(g) = g_consumer.dequeue() {
            x_points.copy_within(0..PRINT_LEN - 1, 1);
            x_points[0] = (0., g.x as f32);
            y_points.copy_within(0..PRINT_LEN - 1, 1);
            y_points[0] = (0., g.y as f32);
            z_points.copy_within(0..PRINT_LEN - 1, 1);
            z_points[0] = (0., g.z as f32);
            for index in 0..PRINT_LEN {
                x_points[index].0 += 1.;
                y_points[index].0 += 1.;
                z_points[index].0 += 1.;
            }

            term.move_cursor_to(0, 0).unwrap();
            Chart::new(200, 100, 0., 1000.)
                .linecolorplot(&Shape::Lines(&x_points), RGB8::new(0xff, 0x00, 0x00))
                .linecolorplot(&Shape::Lines(&y_points), RGB8::new(0x00, 0xff, 0x00))
                .linecolorplot(&Shape::Lines(&z_points), RGB8::new(0x00, 0x00, 0xff))
                .display();
            // println!("{:#?}", &points);
        }

        if should_run.as_ref().load(Ordering::Acquire) == false {
            break;
        }
    }
}
