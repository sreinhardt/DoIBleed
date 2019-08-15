### Do I Bleed?

##### Exploitation
##### 100-200 points

Network services are tricky things to build, especially when trusting user input. We found this service running that is having trouble with basic math. Could you help it with some answers? Maybe it'll provide a lengthy flag!

Try at `f80ac865e59fcd25817722efd0225048.bsides.40bytectf.com:22380`

Flag: `40ByteCTF{1_c@n-h4z=Th3_m3M0rI3s?}`

#### Getting running

```sh
# Install rustup
sudo apt-get install build-essential software-properties-common -y
curl https://sh.rustup.rs -sSf | sh
rustup default nightly
# build and generate md5sum for dns and client binary
cd doibleed/
cargo build --release
strip target/release/doibleed
md5sum ./target/release/doibleed
cp ./target/release/doibleed ~/doibleed-noflag
# Build executable
cargo build --release --features=real_flag
# add user
useradd -m -U doibleed -s /bin/false
# copy binary to directory
cp ./target/release/doibleed /home/doibleed/
chown doibleed:doibleed /home/doibleed/doibleed/
chmod 540 /home/doibleed/doibleed
```
* Add systemd service `/etc/systemd/system/doibleed.service`
  ```
  [Unit]
  Description=Executes DoIBleed CTF challenge
  Wants=network.target
  After=network.target

  [Service]
  User=doibleed
  Group=doibleed
  WorkingDirectory=~
  ExecStart=/home/doibleed/doibleed
  Restart=always
  RestartSec=10

  [Install]
  WantedBy=multi-user.target
  ```
* Start and enable
  ```sh
  sudo systemctl start doibleed
  sudo systemctl enable doibleed
  ```