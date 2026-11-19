//! Human-readable output.

use nothing_link::{Candidate, Status};
use nothing_proto::{BassEnhance, Battery, BatteryLevel, CustomEq, EarFit, FitResult, Gesture, Side};

pub fn on_off(enabled: bool) -> &'static str {
    if enabled { "on" } else { "off" }
}

pub fn battery_level(level: Option<BatteryLevel>) -> String {
    match level {
        Some(l) if l.charging => format!("{}% (charging)", l.percent),
        Some(l) => format!("{}%", l.percent),
        None => "-".into(),
    }
}

pub fn bass(bass: &BassEnhance) -> String {
    if bass.enabled {
        format!("on, level {}/{}", bass.level, BassEnhance::MAX_LEVEL)
    } else {
        "off".into()
    }
}

pub fn custom_eq(eq: &CustomEq) -> String {
    format!(
        "bass {:+.1} dB, mid {:+.1} dB, treble {:+.1} dB",
        eq.bass_db, eq.mid_db, eq.treble_db
    )
}

pub fn ear_fit(fit: &EarFit) -> String {
    let describe = |result: FitResult| match result {
        FitResult::Good => "good seal",
        FitResult::Poor => "poor seal, try another ear tip size",
    };
    format!("Left:  {}\nRight: {}", describe(fit.left), describe(fit.right))
}

pub fn print_devices(devices: &[Candidate]) {
    if devices.is_empty() {
        println!("No paired Nothing earbuds.");
    }
    for device in devices {
        let state = if device.connected {
            "connected"
        } else {
            "not connected"
        };
        println!("{}  {}  ({state})", device.address, device.name);
    }
}

pub fn print_battery(battery: &Battery) {
    println!("Left:  {}", battery_level(battery.left));
    println!("Right: {}", battery_level(battery.right));
    println!("Case:  {}", battery_level(battery.case));
}

pub fn print_gestures(gestures: &[Gesture]) {
    for side in Side::ALL {
        println!("{}:", side.label());
        for gesture in gestures.iter().filter(|g| g.side == side) {
            println!("  {:<24} {}", gesture.kind.label(), gesture.action.label());
        }
    }
}

pub fn print_status(status: &Status) {
    fn row(label: &str, value: Option<String>) {
        if let Some(value) = value {
            println!("{label:<20}{value}");
        }
    }

    println!("{} ({}, {})", status.name, status.model_code, status.address);
    row("Firmware", status.firmware.clone());
    row("Serial number", status.serial.clone());
    if let Some(battery) = &status.battery {
        println!("Battery");
        println!("  {:<18}{}", "Left", battery_level(battery.left));
        println!("  {:<18}{}", "Right", battery_level(battery.right));
        println!("  {:<18}{}", "Case", battery_level(battery.case));
    }
    row("Noise control", status.anc.map(|mode| mode.label().to_owned()));
    row("EQ preset", status.eq_preset.map(|preset| preset.label()));
    row("Custom EQ", status.custom_eq.as_ref().map(custom_eq));
    row("Bass enhance", status.bass.as_ref().map(bass));
    row("Low latency", status.low_latency.map(|v| on_off(v).to_owned()));
    row(
        "In-ear detection",
        status.in_ear_detection.map(|v| on_off(v).to_owned()),
    );
    if let Some(gestures) = &status.gestures {
        println!("Gestures");
        for side in Side::ALL {
            for gesture in gestures.iter().filter(|g| g.side == side) {
                println!(
                    "  {:<7}{:<24}{}",
                    side.label(),
                    gesture.kind.label(),
                    gesture.action.label()
                );
            }
        }
    }
}
