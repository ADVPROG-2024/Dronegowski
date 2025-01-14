# Dronegowski

This repository is part of the **Advanced Programming** project. The `Dronegowski` drone implementation can be found in
`src/drone.rs`. It adheres to the specifications outlined in the protocol document
available [here](https://github.com/WGL-2024/WGL_repo_2024/blob/main/AP-protocol.md). The drone is fully compliant with
the required functionalities and correctly implements the `Drone` trait, providing the `new(...)` constructor and the
`run()` method. Once properly imported, it can be utilized in simulation environments.

## Importing the Dronegowski Drone

To include the `Dronegowski` drone in your project, add the following to your `Cargo.toml` file:

```toml
[dependencies]
dronegowski_drone = { git = "https://github.com/ADVPROG-2024/Dronegowski.git" }
```

To use it in your Rust code, import the `Dronegowski` struct:

```rust
use dronegowski_drone::Dronegowski;
```
