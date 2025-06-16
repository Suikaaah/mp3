# mp3
An optimized MP3 player with smooth/dynamic speed controls

![thumbnail](https://github.com/Suikaaah/mp3/blob/main/thumbnail.png)

## Usage
`cargo run --release -- <your mp3 folder>`

Note: Files will be collected recursively.

## Build/Run
### Linux
- Install `libsdl2-dev` and `libsdl2-ttf-dev`

### Windows
- Download the development releases [SDL2](https://github.com/libsdl-org/SDL/releases/tag/release-2.32.8) | [SDL2_ttf](https://github.com/libsdl-org/SDL_ttf/releases/tag/release-2.24.0)
- Copy `*.lib` to the library directory of your Rust compiler
- Copy `*.dll` to `System32` or to the directory where your executable will be
