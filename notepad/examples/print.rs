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
    types::{self, AccelerationVec, GravityVec},
};

static mut A_QUEUE: Queue<AccelerationVec, { types::ACC_QUEUE_SIZE }> = Queue::new();
static mut G_QUEUE: Queue<GravityVec, { types::GRAV_QUEUE_SIZE }> = Queue::new();

fn main() {
    let should_run = Arc::new(AtomicBool::new(true));
    let should_run_thread_ref = should_run.clone();
    let mut serial = comms::PenselSerial::new_first_matching();

    let (a_producer, mut a_consumer) = unsafe { A_QUEUE.split() };
    let (g_producer, mut g_consumer) = unsafe { G_QUEUE.split() };

    let _sender = thread::spawn(move || {
        serial.parse_data_until(a_producer, g_producer, should_run_thread_ref)
    });

    should_run.as_ref().store(true, Ordering::Release);

    loop {
        if let Some(a) = a_consumer.dequeue() {
            println!("A: {:?}", a);
        }
        if let Some(g) = g_consumer.dequeue() {
            println!("G: {:?}", g);
        }
    }
}
