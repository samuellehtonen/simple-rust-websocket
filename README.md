# Simple Rust WebSocket
A Simple WebSocket implementation written in Rust.

This is a super light weight WebSocket which is created for watching a very small file for changes and send new contents to a client as soon as the file contents change. This is used in a
[sahkotutka](https://sahkotutka.fi) website to update users with fresh data every hour.

## Compile
Tested with rustc 1.85.1
```
cargo build
```

## Set up

### Run as a service in a Linux server

edit /etc/systemd/system/simple-websocket-daemon.service
```
[Unit]
Description=WebSocket File Watcher Daemon
After=network.target

[Service]
ExecStart=/path/to/target/release/your-binary-name
Restart=always
User=your-username
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
```

### Start service
```
sudo systemctl daemon-reexec
sudo systemctl daemon-reload
sudo systemctl enable simple-websocket-daemon
sudo systemctl start simple-websocket-daemon
```
Check the logs to verify your webSocket is running 
```
journalctl -u simple-websocket-daemon -f
```
### Apache2 configuration

This is required for https redirect.

Enable required mods
```
a2enmod proxy
a2enmod proxy_wstunnel
systemctl restart apache2
```

In your site config
```
<VirtualHost *:443>
    # Your regular web config...

    # WebSocket Proxy
    ProxyPass "/ws" "ws://localhost:8080/ws"
    ProxyPassReverse "/ws" "ws://localhost:8080/ws"

    # Your regular web config...
</VirtualHost>
```
