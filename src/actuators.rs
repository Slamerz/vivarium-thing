//! Physical relay outputs that drive the fans and misting pump.
//!
//! [`RelayController`](crate::relays::RelayController) decides *what* the relays
//! should do and exposes that intent as a [`RelayOutputs`]. This module owns the
//! actual GPIO pins and is the only place that touches the hardware, keeping the
//! control logic free of any board-specific pin types.
//!
//! Wiring (unchanged from the original Arduino layout):
//! - left fan  -> digital pin D9
//! - right fan -> digital pin D10
//! - pump      -> digital pin D8
//!
//! The relay boards are active-high: a logic high closes the relay and energises
//! the attached device.

use crate::relays::RelayOutputs;
use embedded_hal::digital::OutputPin;

/// Holds the three relay GPIOs. Generic over [`OutputPin`] so it does not depend
/// on the concrete `arduino-hal` pin types (each pin has a distinct type), which
/// keeps this struct testable and the wiring described in one spot.
pub struct Actuators<LeftFan, RightFan, Pump> {
    left_fan: LeftFan,
    right_fan: RightFan,
    pump: Pump,
}

impl<LeftFan, RightFan, Pump> Actuators<LeftFan, RightFan, Pump>
where
    LeftFan: OutputPin,
    RightFan: OutputPin,
    Pump: OutputPin,
{
    /// Takes ownership of the already-configured output pins.
    pub fn new(left_fan: LeftFan, right_fan: RightFan, pump: Pump) -> Self {
        Self {
            left_fan,
            right_fan,
            pump,
        }
    }

    /// Drives every relay to match the controller's desired state in one call.
    ///
    /// `bool` converts directly into `PinState` (`true` -> high -> relay closed),
    /// so a single mapping per pin replaces the previous if/else blocks. Pin
    /// writes are infallible on AVR, so the `Result` is intentionally ignored.
    pub fn apply(&mut self, outputs: RelayOutputs) {
        let _ = self.left_fan.set_state(outputs.left_fan.into());
        let _ = self.right_fan.set_state(outputs.right_fan.into());
        let _ = self.pump.set_state(outputs.pump.into());
    }
}
