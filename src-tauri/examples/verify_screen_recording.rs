//! Records the USB-connected iPhone's screen through the production capture
//! path, cuts a middle section out, and exports the rest as MP4.
//!
//! ```text
//! cargo run --manifest-path src-tauri/Cargo.toml --example verify_screen_recording -- [seconds] [device name]
//! ```
//!
//! Like the app, it records on a worker thread while the main thread runs its
//! run loop: CoreMediaIO announces iOS devices through the main run loop.
//! Writes `recording.mov` and `export.mp4` under a fresh temporary directory
//! and prints what each file contains. Nothing on the device is changed.

use std::{
    ffi::c_void,
    sync::mpsc,
    time::{Duration, Instant},
};

use idevice_desktop_lib::screen_capture::{self, RecorderMessage};

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFRunLoopDefaultMode: *const c_void;
    fn CFRunLoopRunInMode(mode: *const c_void, seconds: f64, return_after_source: u8) -> i32;
}

fn main() {
    let mut args = std::env::args().skip(1);
    let seconds: u64 = args
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8);
    let wanted = args.next();
    let worker = std::thread::spawn(move || verify(seconds, wanted));
    while !worker.is_finished() {
        // SAFETY: runs the main thread's run loop for a quarter second.
        unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.25, 0) };
    }
    if worker.join().is_err() {
        std::process::exit(1);
    }
}

fn verify(seconds: u64, wanted: Option<String>) {
    let directory =
        std::env::temp_dir().join(format!("verify-screen-recording-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("create output directory");
    let source = directory.join("recording.mov");

    let (sender, receiver) = mpsc::channel();
    let stopper = sender.clone();
    let requested = Instant::now();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(seconds) + Duration::from_secs(3));
        let _ = stopper.send(RecorderMessage::Stop);
    });
    let movie = screen_capture::record(
        wanted.as_deref(),
        &source,
        sender,
        &receiver,
        &|device| {
            println!(
                "recording started after {:.1}s on {device}",
                requested.elapsed().as_secs_f64()
            )
        },
        &|| false,
    )
    .unwrap_or_else(|error| panic!("recording failed: {error:?}"));
    println!(
        "recorded {:.2}s, {}x{}, {} bytes from {} -> {}",
        movie.duration_ms as f64 / 1000.0,
        movie.width,
        movie.height,
        movie.bytes,
        movie.device_name,
        source.display()
    );

    let third = movie.duration_ms / 3;
    let segments = [(0, third), (third * 2, movie.duration_ms)];
    let destination = directory.join("export.mp4");
    let last_reported = std::cell::Cell::new(-1.0);
    screen_capture::export(
        &source,
        &destination,
        &segments,
        &|progress| {
            if progress - last_reported.get() >= 0.25 {
                last_reported.set(progress);
                println!("export {:.0}%", progress * 100.0);
            }
        },
        &|| false,
    )
    .unwrap_or_else(|error| panic!("export failed: {error:?}"));
    let exported = screen_capture::inspect(&destination).expect("inspect export");
    println!(
        "exported {:.2}s (expected about {:.2}s), {}x{}, {} bytes -> {}",
        exported.duration_ms as f64 / 1000.0,
        (movie.duration_ms - third) as f64 / 1000.0,
        exported.width,
        exported.height,
        exported.bytes,
        destination.display()
    );
}
