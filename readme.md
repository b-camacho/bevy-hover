# bevy-hover
Adds mouse hover events for your 3d bevy meshes. Check out the [live wasm demo](https://chmod.site/static/bevy-hover/index.html).

[![demo](https://github.com/b-camacho/bevy-hover/assets/12277070/58a56e45-6ad0-469a-a08b-16c62ec75010)](https://github.com/b-camacho/bevy-hover/assets/12277070/0eeac20b-1b98-4164-a909-a90547352b48)

## How to use?
check out `examples/main`

## Testing?
haha maybe. for now `cargo run --example main` and give it a good once over

## Todo
- [x] events for hover start/end
- [x] fade out hover color for demo
- [x] static image / video for demo
- [x] ortho camera support
- [x] fix multiple hovers for occluded items
- [ ] publish the crate
- [x] click support (click start/env event)
- [ ] benchmark
- [ ] add spatial index or similar
- [ ] explain how ortho camera ray direction is equal to 3rd column of xform matrix * -1
