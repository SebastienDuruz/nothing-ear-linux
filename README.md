# nothing-ear-linux

Control **Nothing** earbuds from Linux: noise control, equalizer, bass enhance, gestures, battery… without a phone.

Built and tested with **Nothing Ear (B171)** on KDE Plasma (Wayland).

> Audio already works on Linux with nothing to install, thanks to BlueZ and PipeWire. This project adds the Nothing-specific settings that are normally only available in the Nothing X phone app.

## What you get

| Program | What it does |
|---|---|
| `nothing-ear` | Tray icon with quick settings, and a window with every setting |
| `nothing-ctl` | The same settings from the command line, with JSON output for scripts |

## Requirements

- Rust 1.85 or newer, and `make`
- BlueZ, D-Bus and the `dbus-1` headers (the `dbus` package on Arch / CachyOS)
- Earbuds already paired and connected over Bluetooth

## Install

```bash
make install
```

Binaries go to `~/.local/bin` and a launcher is added to the application menu. To start the tray icon at login, add `nothing-ear --hidden` to your desktop's autostart list (KDE: System Settings → Autostart).

To remove everything:

```bash
make uninstall
```

Run `make` on its own to list every available command.

## Usage

### Tray app

Start **Nothing earbuds** from the application menu, or run `nothing-ear`. Left-click the tray icon to open the settings window. Right-click it for quick settings: noise control, equalizer, bass enhance, low latency, in-ear detection.

The app reconnects on its own when the earbuds come back. It also follows changes made on the earbuds themselves, such as switching noise control with a gesture, and battery updates.

### Command line

```bash
nothing-ctl status                         # every setting
nothing-ctl battery
nothing-ctl anc transparency               # off, transparency, adaptive, high, mid, low
nothing-ctl eq more-bass                   # balanced, more-bass, more-treble, voice
nothing-ctl custom-eq --bass 3 --treble -1
nothing-ctl bass 4                         # off, or a level from 1 to 5
nothing-ctl latency on
nothing-ctl in-ear off
nothing-ctl gestures
nothing-ctl gesture right hold noise-cycle
nothing-ctl ring left --yes                # take the earbuds out of your ears first!
nothing-ctl fit-test
nothing-ctl --json status                  # JSON output
nothing-ctl -v battery                     # log every frame sent and received
```

Run `nothing-ctl --help` or `nothing-ctl <command> --help` for details.

## Feature status

Every read was verified on real Nothing Ear (B171). So were the writes of **low latency** and **noise control**. The other writes follow community reverse engineering and still need to be confirmed. [docs/PROTOCOL.md](docs/PROTOCOL.md) gives the details, command by command.

## Known limitations

- The earbuds accept only one control connection at a time. While `nothing-ear` runs, `nothing-ctl` cannot connect.
- The case only reports its battery while its lid is open with an earbud inside.
- Only the B171 has a precise feature list. Other Nothing and CMF models get a conservative one.

## For developers

### Code map

```
crates/
├── nothing-proto/      Protocol only, no I/O. Pure functions, unit tested.
│   └── src/
│       ├── frame.rs        Frame format, CRC, stream decoder
│       ├── command.rs      Command codes
│       ├── model.rs        Known models and their features
│       └── settings/       One file per setting: type + parse() + encode()
├── nothing-link/       Bluetooth transport (BlueZ through the `bluer` crate)
│   └── src/
│       ├── discovery.rs    Find paired Nothing earbuds
│       ├── earbuds.rs      Open the channel; one method per setting
│       ├── client.rs       Request/reply matching, notifications
│       └── status.rs       Snapshot of every setting
├── nothing-ctl/        Command-line tool
│   └── src/
│       ├── args.rs         Arguments (clap)
│       ├── print.rs        Human-readable output
│       └── main.rs         One match arm per subcommand
└── nothing-tray/       Tray app (binary `nothing-ear`)
    └── src/
        ├── main.rs         Threads and wiring
        ├── state.rs        Shared state and the Action enum
        ├── worker.rs       Owns the connection, applies actions
        ├── tray.rs         Tray icon and menu (ksni)
        ├── window.rs       Settings window (egui): Sound, Gestures and Device tabs
        ├── widgets.rs      Reusable widgets: card, switch, segmented buttons
        ├── logo.rs         App logo (dot grid): tray, window and menu icons
        ├── theme.rs        Look: fonts (assets/fonts), black and white colours, text sizes
        └── demo.rs         Demo mode: sample data, no Bluetooth
```

### How a change flows

```
tray.rs / window.rs ──Action──▶ worker.rs ──▶ nothing-link (Earbuds) ──▶ RFCOMM
        ▲                           │
        └──── state.rs (Shared) ◀───┘  the worker writes, reads back, then stores the result
```

The UI never talks to the earbuds directly. It sends an `Action`; the worker applies it, reads the setting back and stores what the earbuds report in the shared state.

### Adding a setting

1. **Protocol**: add the command codes to `nothing-proto/src/command.rs`, and a type with `parse()` / `encode()` in `nothing-proto/src/settings/`. Add a test with a real payload if you have one.
2. **Transport**: add a getter and a setter in `nothing-link/src/earbuds.rs`, and a field in `status.rs` (read it in `Earbuds::status`).
3. **CLI**: add a subcommand in `nothing-ctl/src/args.rs` and its match arm in `main.rs`.
4. **App**: add an `Action` variant in `state.rs`, handle it in `worker.rs::apply`, and draw it in `window.rs` (and `tray.rs` if it belongs in the quick menu).

### Commands

| Command | What it does |
|---|---|
| `make build` | Debug build (`make install` does its own optimised build) |
| `make run` | Start the tray app from the sources (`RUST_LOG=debug make run` for logs) |
| `make test` | Unit tests |
| `make fmt` | Format the code (`rustfmt.toml`: 110 columns) |

To run the CLI from the sources: `cargo run -p nothing-ctl -- status`.

To work on the window without earbuds, start it in demo mode: `NOTHING_EAR_DEMO=1 make run`. It shows sample data, needs no Bluetooth and does not take the control channel from a running instance.

Install paths can be changed: `make install PREFIX=/usr/local`, or `make install DESTDIR=/tmp/pkg` to stage a package.

## Credits

The protocol was decoded by the community, in particular by [ear-web](https://github.com/radiance-project/ear-web), [EarA-linux](https://github.com/oneydef/EarA-linux) and [ear-native](https://github.com/dest4590/ear-native). This repository does not reuse their code: it reimplements the protocol from its description and from captures made on real earbuds.

The window uses the [Space Mono](https://github.com/googlefonts/spacemono) and [Doto](https://github.com/oliverlalan/Doto) fonts, under the SIL Open Font License (see `crates/nothing-tray/assets/fonts`).

This project is not affiliated with Nothing Technology Limited.

## License

MIT, see [LICENSE](LICENSE). The bundled fonts keep their own license, the SIL Open Font License (see Credits).
