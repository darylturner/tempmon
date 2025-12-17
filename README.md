# tempmon

A lightweight Rust application for monitoring DS18B20 temperature probes on Raspberry Pi and exposing metrics via Prometheus.

## Features

- Automatic discovery of DS18B20 temperature sensors on the 1-wire bus
- Configurable temperature resolution (9-12 bits)
- Friendly naming for sensors via configuration
- Prometheus metrics exporter
- Error tracking and reporting
- Low resource footprint suitable for embedded systems

## Hardware Requirements

- Raspberry Pi (any model with GPIO)
- DS18B20 temperature sensor(s)
- 4.7kΩ resistor (pull-up resistor for 1-wire bus)

## Raspberry Pi Setup

### Enable 1-Wire Protocol

1. **Edit the boot configuration:**
   ```bash
   sudo vim /boot/config.txt
   ```

2. **Add the following line:**
   ```
   dtoverlay=w1-gpio
   ```

3. **Make the w1-therm module load at boot:**
   ```bash
   echo "w1-therm" | sudo tee -a /etc/modules
   ```

   Alternatively, create a dedicated config file:
   ```bash
   echo "w1-therm" | sudo tee /etc/modules-load.d/w1.conf
   ```

4. **Reboot the Raspberry Pi:**
   ```bash
   sudo reboot
   ```

5. **Verify modules are loaded:**
   ```bash
   lsmod | grep w1
   ```
   You should see `w1_gpio` and `w1_therm` listed.

6. **Verify sensors are detected:**
   ```bash
   ls /sys/bus/w1/devices/
   ```
   You should see entries like `28-xxxxxxxxxxxx` for each DS18B20 sensor.

### Wiring DS18B20 Sensors

Standard connection (using GPIO4/pin 7):
- Red wire (VDD) → 3.3V (pin 1)
- Black wire (GND) → Ground (pin 6)
- Yellow wire (DATA) → GPIO4 (pin 7)
- 4.7kΩ resistor between VDD and DATA

## Installation

### Pre-built Binary

1. Copy the compiled binary to your Raspberry Pi:
   ```bash
   scp target/arm-unknown-linux-musleabihf/release/tempmon pi@your-pi:/usr/local/bin/
   ```

2. Make it executable:
   ```bash
   sudo chmod +x /usr/local/bin/tempmon
   ```

### Building from Source

Prerequisites:
- Rust toolchain
- `cargo-zigbuild` for cross-compilation

**Install cargo-zigbuild:**
```bash
cargo install cargo-zigbuild
```

**Add the appropriate Rust target for your Raspberry Pi model:**

| Raspberry Pi Model | Architecture | Rust Target |
|-------------------|--------------|-------------|
| Pi Zero, Zero W, Pi 1 | ARMv6 | `arm-unknown-linux-musleabihf` |
| Pi 2, 3, 4 (32-bit) | ARMv7 | `armv7-unknown-linux-musleabihf` |
| Pi 3, 4 (64-bit OS) | ARMv8/AArch64 | `aarch64-unknown-linux-musl` |

```bash
# For Pi Zero/Pi 1 (ARMv6) - default target in Makefile
rustup target add arm-unknown-linux-musleabihf

# For Pi 2/3/4 (ARMv7)
rustup target add armv7-unknown-linux-musleabihf

# For Pi 3/4 64-bit
rustup target add aarch64-unknown-linux-musl
```

**Build:**
```bash
# Default build (ARMv6 - Pi Zero/Pi 1)
make build

# Or specify target directly for ARMv7 (Pi 2/3/4):
cargo zigbuild --target=armv7-unknown-linux-musleabihf --release

# Or for 64-bit (Pi 3/4):
cargo zigbuild --target=aarch64-unknown-linux-musl --release
```

**Note:** The ARMv6 target (`arm-unknown-linux-musleabihf`) will work on all models but may not be optimized for newer Pis. Use the specific target for your hardware for best performance.

## Configuration

1. **Create the configuration directory:**
   ```bash
   sudo mkdir -p /etc/tempmon
   ```

2. **Copy the example configuration:**
   ```bash
   sudo cp config.toml.example /etc/tempmon/config.toml
   ```

3. **Edit the configuration:**
   ```bash
   sudo vim /etc/tempmon/config.toml
   ```

### Configuration Options

```toml
[settings]
# Port for Prometheus metrics endpoint
metrics_port = 9184

# Interval between temperature readings (seconds)
probe_interval = 15

# DS18B20 resolution: 9, 10, 11, or 12 bits
# 9  = 0.5°C    (~94ms conversion)
# 10 = 0.25°C   (~188ms conversion)
# 11 = 0.125°C  (~375ms conversion)
# 12 = 0.0625°C (~750ms conversion)
probe_resolution = 10

[probe_labels]
# Map hardware IDs to friendly names
# Find your probe IDs: ls /sys/bus/w1/devices/
"28-0123456789ab" = "basking_spot"
"28-0123456789cd" = "cool_side"
```

### Finding Probe IDs

To discover your connected sensor IDs:
```bash
ls /sys/bus/w1/devices/
```

Look for entries starting with `28-` (DS18B20 family code).

## Usage

### Run Directly

```bash
sudo tempmon
```

The application will:
1. Load configuration from `/etc/tempmon/config.toml`
2. Discover all DS18B20 sensors
3. Set the configured resolution for each sensor
4. Start the Prometheus metrics endpoint on the configured port
5. Begin reading temperatures at the configured interval

### Run as a Systemd Service

Create `/etc/systemd/system/tempmon.service`:

```ini
[Unit]
Description=Temperature Monitor
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/tempmon
Restart=always
RestartSec=10
User=root

[Install]
WantedBy=multi-user.target
```

Enable and start the service:
```bash
sudo systemctl enable tempmon
sudo systemctl start tempmon
sudo systemctl status tempmon
```

View logs:
```bash
sudo journalctl -u tempmon -f
```

## Prometheus Integration

### Metrics Exposed

- `dash_temp_readings{probe="<name>"}` - Current temperature in Celsius
- `dash_temp_read_errors_total{probe="<name>",error_type="<type>"}` - Total read errors

### Prometheus Configuration

Add to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'tempmon'
    static_configs:
      - targets: ['<raspberry-pi-ip>:9184']
```

### Testing the Endpoint

```bash
curl http://<raspberry-pi-ip>:9184/metrics
```

## Troubleshooting

### No sensors detected
- Check wiring and pull-up resistor
- Verify 1-wire is enabled: `lsmod | grep w1`
- Check `/sys/bus/w1/devices/` for sensor entries

### Permission denied errors
- Run with `sudo` (requires root access to modify sensor resolution)
- Check file permissions on `/sys/bus/w1/devices/`

### Port already in use
- Check if another process is using the metrics port: `sudo netstat -tulpn | grep 9184`
- Change `metrics_port` in config.toml

### Inconsistent readings
- Increase `probe_resolution` for better accuracy
- Check sensor wiring and connections
- Verify power supply is stable

## License

[Add your license information here]
