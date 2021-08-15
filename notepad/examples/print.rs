//! Example that just prints all packets
use heapless::spsc::Queue;
use std::{sync::atomic::AtomicBool, thread};

use notepad::{
    comms,
    types::{AccelerationVec, GravityVec},
};

static mut A_QUEUE: Queue<AccelerationVec, 4> = Queue::new();
static mut G_QUEUE: Queue<GravityVec, 4> = Queue::new();

fn main() {
    let should_run = AtomicBool::new(true);
    let mut serial = comms::PenselSerial::new_first_matching();

    let (a_producer, mut a_consumer) = unsafe { A_QUEUE.split() };
    let (g_producer, mut g_consumer) = unsafe { G_QUEUE.split() };

    let _sender =
        thread::spawn(move || serial.parse_data_until(a_producer, g_producer, &should_run));

    loop {
        if let Some(a) = a_consumer.dequeue() {
            println!("A: {:?}", a);
        }
        if let Some(g) = g_consumer.dequeue() {
            println!("G: {:?}", g);
        }
    }
}
