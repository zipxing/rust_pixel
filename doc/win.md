# Windows(win11-wsl2-ubuntu) install guide

**Install wsl2 and ubuntu**
```
wsl --install
```
then open windows terminal with ubuntu.


**Install Nerd Font**

```
curl -sS https://webi.sh/nerdfont | sh
Terminal: Preferences > Text > Custom font > DroidSansMono Nerd Font
```

**Install rust**
``` 
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"            # For sh/bash/zsh/ash/dash/pdksh
``` 

**Install wasm-pack**
```
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

**Install some dependent libraries and software**
``` 
sudo apt update
sudo apt-get install git
sudo apt-get install gcc
sudo apt-get install ffmpeg            # Used to convert gif to ssf sequence frame files(.ssf)
sudo apt-get install libsdl2-dev
sudo apt-get install libsdl2-image-dev
sudo apt-get install libsdl2-gfx-dev
``` 

**Download RustPixel and deploy cargo-pixel**
``` 
git clone https://github.com/zipxing/rust_pixel
cd rust_pixel
cargo install --path tools/cargo-pixel --root ~/.cargo
``` 
