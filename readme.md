# Long-Term Physics Project

## Quick Start

Install Rust and run `Cargo run` in the repository.

### wasm start

```rust
cargo install wasm-pack
wasm-pack build --release --target web
```

Open index.html.

### issues

### physics engine roadmap

1. Draw a ball drop down by the gravity, collide with the floor and bounce up.
2. With the air friction, it will finally turn into a still state.
3. consider angle speed and torque

## TODO

### big 

- [X] camera simple movement
- [X] fixed 60 frames per second
- [X] a simple render engine
- [X] simple GUI, show some immediate infos
- [X] render a simple sphere
- [ ] a simple physics engine
- [ ] water simulation (both physics and rendering)
  - [ ] rigid body simulation
    - [ ] a super ball considering torque
  - [ ] implict eular
  - [ ] RK4
  - [ ] position-based / verlet integration
- [ ] add grids rendering
- [ ] try some new techs
- [ ] a ball considering torque
- [ ] water simulation (both physics and rendering)
- [ ] support skeleton and skeleton animation (simplest way)
- [ ] a timewheel to put all things together (simplest way)
- [ ] a physical-driven vehicle, including 4 wheels and 2 wheels

### physics

- [ ] a static plane and a dynamic cylinder. the cylinder drop down and bounce up, ending up with a still state.
