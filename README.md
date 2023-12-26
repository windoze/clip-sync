# ClipSync

## Build

* Windows and macOS: `cargo build --release`
* Linux:
    * `apt install libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev`
    * `cargo build --release`

## Run

Setup MQTT broker and run `./target/release/clip-sync` with the following arguments:

    * `-a` or `--mqtt-server-addr`: MQTT broker address
    * `-p` or `--mqtt-server-port`: MQTT broker port
    * `-u` or `--mqtt-username`: MQTT username, can be omitted if no authentication is required
    * `-w` or `--mqtt-password`: MQTT password, can be omitted if no authentication is required
    * `-t` or `--mqtt-topic`: MQTT topic, defaults to `clipboard`
    * `-c` or `--mqtt-client-id`: Client id, defaults to the hostname

## Usage

To automatically start the program on system startup:

* Windows:
    Create a shortcut to `clip-sync.exe` with required arguments in the startup folder.
* macOS:
    Update the `com.0d0a.clipsync.plist` file with the required arguments and copy it to `~/Library/LaunchAgents/`.
    Then run `launchctl load ~/Library/LaunchAgents/com.0d0a.clipsync.plist`.
* Linux:
    Update the `clip-sync.desktop` file with the required arguments and copy it to `~/.config/autostart/`.