# ClipSync

## Build

* Windows and macOS: `cargo build --release`
* Linux:
    * `apt install libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev`
    * `cargo build --release`

## Run

1. Setup MQTT broker

2. Create config file `config.toml` at the default config path (`~/.config/clip-sync/config.toml` on Linux, `%APPDATA%\clip-sync\config.toml` on Windows, `~/Library/Application Support/clip-sync/config.toml` on macOS) with the following settings:

    * `mqtt-server-addr`: MQTT broker address
    * `mqtt-server-port`: MQTT broker port
    * `mqtt-username`: MQTT username, can be omitted if no authentication is required
    * `mqtt-password`: MQTT password, can be omitted if no authentication is required
    * `mqtt-topic`: MQTT topic, defaults to `clipboard`
    * `mqtt-client-id`: Client id, defaults to the hostname

3. Run `clip-sync`.

## Usage

To automatically start the program on system startup:

* Windows:
    Create a shortcut to `clip-sync.exe` in the startup folder.
* macOS:
    Update the `com.0d0a.clipsync.plist` with the correct path and copy it to `~/Library/LaunchAgents/`.
    Then run `launchctl load ~/Library/LaunchAgents/com.0d0a.clipsync.plist`.
* Linux:
    Update the `clip-sync.desktop` file with the correct path and copy it to `~/.config/autostart/`.