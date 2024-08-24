# MacOS install guide

**Install iTerm2 & Nerd Font**

For better terminal display, you need to install [iTerm2] and set it to dark background mode. 
```
#iTerm2 : Settings... > Profiles > Colors > Color presets... > Dark Background
```

You also need to install [DroidSansMono Nerd Font].
```
curl -sS https://webi.sh/nerdfont | sh
#iTerm2 : Settings... > Profiles > Text > Font > DroidSansMono Nerd Font
```

[iTerm2]: https://iterm2.com/
[DroidSansMono Nerd Font]: https://github.com/ryanoasis/nerd-fonts

**Install brew**
``` 
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
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
brew install ffmpeg            # Used to convert gif to ssf sequence frame files(.ssf)
brew install sdl2
brew install sdl2_image
brew install sdl2_gfx
brew install sdl2_ttf
brew install sdl2_mixer
``` 

Tips: Missing path in LIBRARY_PATH environment variable<br>
The Homebrew package manager symlinks library to the directory /usr/local/lib. <br>
To use these libraries with Rust, you need to add it to the LIBRARY_PATH environment variable. <br>
The command echo $LIBRARY_PATH will tell you if /usr/local/lib is added. <br>
If it is missing, add the following to the ~/.bash_profile configuration file:
```
export LIBRARY_PATH="$LIBRARY_PATH:/usr/local/lib"
```
This will add the directory to the environment variable each time you start up a new Terminal window.


**Download RustPixel and deploy cargo-pixel**
``` 
git clone https://github.com/zipxing/rust_pixel
cd rust_pixel
cargo install --path tools/cargo-pixel --root ~/.cargo
``` 
