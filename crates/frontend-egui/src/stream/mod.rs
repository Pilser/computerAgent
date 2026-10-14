//! Stream bridge: worker-thread pump feeding the frame loop.
//!
//! Re-export only; see [`pump`].

pub mod pump;

pub use pump::{Pump, PumpEvent};
