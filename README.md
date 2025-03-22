# hit-demo
The demo server for highlight-it

## Installation

Follow these steps to install and configure hit-demo:

### Prerequisites
- Node.js and npm
- Rust and Cargo
- Systemd
- Root access

### Installation Steps

```bash
# Clone the repository
git clone https://github.com/tn3w/hit-demo.git
cd hit-demo

# Install npm packages
npm install

# Build Rust components
cargo build --release

# Copy executable to bin directory
sudo cp target/release/hit-demo /usr/local/bin/

# Set appropriate permissions
sudo chmod 755 /usr/local/bin/hit-demo

# Create service user
sudo useradd -r -s /bin/false hit-demo

# Copy files to appropriate location
sudo mkdir -p /var/lib/hit-demo
sudo cp -r ./* /var/lib/hit-demo/

# Set appropriate ownership and permissions
sudo chown -R hit-demo:hit-demo /var/lib/hit-demo
sudo chmod -R 750 /var/lib/hit-demo

# Setup systemd service
sudo cp hit-demo.service /etc/systemd/system/
sudo chmod 644 /etc/systemd/system/hit-demo.service
sudo systemctl daemon-reload
sudo systemctl enable hit-demo
sudo systemctl start hit-demo
```

### Verification
```bash
sudo systemctl status hit-demo
```
