#  <img src="./icons/app-icon.png" width = "64" height = "64" align=center /> ClipSync

## Build

* Windows and macOS: `cargo build --release`
* Linux:

  * `apt install pkg-config libdbus-1-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev`

  * `cargo build --release`

## Run

1. ClipSync supports 2 type of servers:

    * Setup MQTT broker if you want to use MQTT, or,

    * Include `server` role in one of the devices to use the device as a server, or run `clip-sync` in the headless mode to start a server on a dedicated machine.

        Refer to the "Linux Headless Server" section.

2. Create config file `config.toml` at the default config path (`~/.config/clip-sync/config.toml` on Linux, `C:\Users\%USERNAME%\AppData\Roaming\clip-sync\config.toml` on Windows, `~/Library/Application Support/clip-sync/config.toml` on macOS).

    Refer to [`config.toml`](./config.toml) for the format.

3. Run `clip-sync`.

## Usage

To automatically start the program on system startup:

* Windows:

    Create a shortcut to `clip-sync.exe` in the startup folder at `%appdata%\Microsoft\Windows\Start Menu\Programs\Startup`.

* macOS:

    Update the `com.0d0a.clipsync.plist` with the correct path and copy it to `~/Library/LaunchAgents/`.

    Then run `launchctl load ~/Library/LaunchAgents/com.0d0a.clipsync.plist`.

* Linux:

    Update the `clip-sync.desktop` file with the correct path and copy it to `~/.config/autostart/`.

    Or use `clip-sync.service` to run it as a systemd service.

* Linux Headless Server (No GUI):

    Build the Docker image by running `docker build -t clipsync .` in the project root.

    Then run `docker run -d --restart unless-stopped --name clipsync -p 3000:3000 clipsync`.

    To persist the clipboard history, add `-v /path/to/index/storage:/index` to the `docker run` command.
