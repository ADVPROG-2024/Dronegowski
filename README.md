# Dronegowski

This repository is part of the **Advanced Programming** project. The `Dronegowski` drone implementation can be found in `src/drone.rs`. It adheres to the specifications outlined in the protocol document available [here](https://github.com/WGL-2024/WGL_repo_2024/blob/main/AP-protocol.md). The drone is fully compliant with the required functionalities and correctly implements the `Drone` trait, providing the `new(...)` constructor and the `run()` method. Once properly imported, it can be utilized in simulation environments.

Additionally, the repository contains numerous **test cases** in the `tests` folder, ensuring the drone's behavior aligns with the protocol and validating its overall functionality.

---

## Unique Features

A distinguishing feature of `Dronegowski` is its **conditional debugging system** (currently under development). This system allows users to enable specific debugging options, providing tailored insights into the drone's behavior. Users can choose which aspects of the drone's functionality they want to observe, such as how it handles different types of packets or reacts to commands.

This feature is particularly helpful for analyzing and understanding individual drone behaviors during testing or in simulation scenarios.

---

## Example Usage: Conditional Debugging

When initiating a test and creating a `Dronegowski` instance, users can activate specific debugging options to track desired behaviors. Here's how it works:

### Step 1: Instantiate the Drone
```rust
let mut my_drone = Dronegowski::new(
    1,                  // Drone ID
    sim_controller_send,
    controller_receive,
    packet_receive.clone(),
    senders,
    0.1,                
);
```

Step 2: Enable Debug Options

Enable specific debugging options, such as tracking Ack packet behavior:

```rust
my_drone.set_debug_option_active(&DroneDebugOption::Ack);
```

Step 3: Observe Debug Outputs

After enabling the option, you will receive detailed information about how the drone interacts with Ack packets:

```rust
[Debug: Ack Enabled]
[Drone 1 - Ack Debug] Ack received and now trying to forward it
[Drone 1 - Ack Debug] Error in sending Ack!
[Drone 1 - Ack Debug] Ack sent to Simulation Controller after error
```

### Advantages of the Debugging System

- Customizable Debugging: Select only the information relevant to your analysis
- Protocol Insight: Understand how the drone handles packets and commands, step-by-step.
- Development Aid: Diagnose and debug potential issues during the implementation of new features or while integrating the drone into larger systems. 




