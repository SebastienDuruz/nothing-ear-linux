//! `nothing-ctl`: control Nothing earbuds from the command line.
//!
//! Each subcommand without a value reads the setting; with a value it
//! writes the setting, then reads it back to show what the earbuds report.

mod args;
mod print;

use anyhow::{Context, Result, bail};
use clap::Parser;
use nothing_link::{ConnectOptions, Earbuds};
use nothing_proto::{BassEnhance, Gesture};
use serde::Serialize;

use crate::args::{Cli, Command};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let log_level = if cli.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Warn
    };
    env_logger::Builder::new()
        .filter_level(log_level)
        .format_timestamp(None)
        .init();

    if let Err(error) = run(cli).await {
        eprintln!("Error: {error:#}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    let json = cli.json;

    // The only command that does not need a connection.
    if let Command::Devices = cli.command {
        let devices = nothing_link::list_devices().await?;
        return show(json, &devices, || print::print_devices(&devices));
    }

    let options = ConnectOptions {
        address: cli.device,
        channel: cli.channel,
    };
    let earbuds = Earbuds::connect(&options).await?;

    match cli.command {
        Command::Devices => unreachable!("handled above"),

        Command::Status => {
            let status = earbuds.status().await;
            show(json, &status, || print::print_status(&status))
        }

        Command::Battery => {
            let battery = earbuds.battery().await?;
            show(json, &battery, || print::print_battery(&battery))
        }

        Command::Anc { mode } => {
            if let Some(mode) = mode {
                earbuds.set_anc(mode.into()).await?;
            }
            let mode = earbuds.anc().await?;
            show(json, &mode, || println!("{}", mode.label()))
        }

        Command::Eq { preset } => {
            if let Some(preset) = preset {
                earbuds.set_eq_preset(preset.into()).await?;
            }
            let preset = earbuds.eq_preset().await?;
            show(json, &preset, || println!("{}", preset.label()))
        }

        Command::CustomEq { bass, mid, treble } => {
            if bass.is_some() || mid.is_some() || treble.is_some() {
                // Only change the bands given on the command line.
                let mut eq = earbuds.custom_eq().await.unwrap_or_default();
                eq.bass_db = bass.unwrap_or(eq.bass_db);
                eq.mid_db = mid.unwrap_or(eq.mid_db);
                eq.treble_db = treble.unwrap_or(eq.treble_db);
                earbuds.set_custom_eq(eq).await?;
            }
            let eq = earbuds.custom_eq().await?;
            show(json, &eq, || println!("{}", print::custom_eq(&eq)))
        }

        Command::Bass { level } => {
            if let Some(level) = level {
                let bass = parse_bass_level(&level, &earbuds).await?;
                earbuds.set_bass(bass).await?;
            }
            let bass = earbuds.bass().await?;
            show(json, &bass, || println!("{}", print::bass(&bass)))
        }

        Command::Latency { state } => {
            if let Some(state) = state {
                earbuds.set_low_latency(state.into()).await?;
            }
            let enabled = earbuds.low_latency().await?;
            show(json, &enabled, || println!("{}", print::on_off(enabled)))
        }

        Command::InEar { state } => {
            if let Some(state) = state {
                earbuds.set_in_ear_detection(state.into()).await?;
            }
            let enabled = earbuds.in_ear_detection().await?;
            show(json, &enabled, || println!("{}", print::on_off(enabled)))
        }

        Command::Gestures => {
            let gestures = earbuds.gestures().await?;
            show(json, &gestures, || print::print_gestures(&gestures))
        }

        Command::Gesture { side, kind, action } => {
            let gesture = Gesture {
                side: side.into(),
                kind: kind.into(),
                action: action.into(),
            };
            earbuds.set_gesture(gesture).await?;
            let gestures = earbuds.gestures().await?;
            show(json, &gestures, || print::print_gestures(&gestures))
        }

        Command::Ring { target, stop, yes } => {
            if !stop && !yes {
                bail!("the ring is very loud. Take the earbuds out of your ears, then run again with --yes");
            }
            earbuds.ring(target.into(), !stop).await?;
            if !json {
                if stop {
                    println!("Ringing stopped.");
                } else {
                    println!("Ringing. Stop with: nothing-ctl ring --stop");
                }
            }
            Ok(())
        }

        Command::FitTest => {
            if !json {
                println!("Test running, keep the earbuds in place (about 10 seconds)...");
            }
            let fit = earbuds.ear_fit_test().await?;
            show(json, &fit, || println!("{}", print::ear_fit(&fit)))
        }

        Command::Raw { command, payload } => {
            let command = u16::from_str_radix(command.trim_start_matches("0x"), 16)
                .context("command must be a hex number, for example c007")?;
            let payload = payload.as_deref().map(parse_hex).transpose()?.unwrap_or_default();
            let reply = earbuds.raw(command, payload).await?;
            let bytes: Vec<String> = reply.payload.iter().map(|b| format!("{b:02x}")).collect();
            println!("reply {:#06x}: {}", reply.command, bytes.join(" "));
            Ok(())
        }
    }
}

/// Prints `value` as JSON, or calls `print_text` for human-readable output.
fn show<T: Serialize>(json: bool, value: &T, print_text: impl FnOnce()) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        print_text();
    }
    Ok(())
}

/// Turns "off" or "1".."5" into a bass enhance setting. "off" keeps the
/// current level so that turning it back on restores it.
async fn parse_bass_level(level: &str, earbuds: &Earbuds) -> Result<BassEnhance> {
    if level == "off" {
        let current_level = earbuds.bass().await.map(|bass| bass.level).unwrap_or(1);
        return Ok(BassEnhance {
            enabled: false,
            level: current_level,
        });
    }
    let level: u8 = level.parse().context("expected \"off\" or a level from 1 to 5")?;
    if !(1..=BassEnhance::MAX_LEVEL).contains(&level) {
        bail!("expected \"off\" or a level from 1 to 5");
    }
    Ok(BassEnhance { enabled: true, level })
}

/// Parses hex bytes, ignoring spaces: "01 07 00" or "010700".
fn parse_hex(text: &str) -> Result<Vec<u8>> {
    let digits: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if digits.len() % 2 != 0 {
        bail!("hex payload has an odd number of digits");
    }
    (0..digits.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&digits[i..i + 2], 16).context("invalid hex payload"))
        .collect()
}
