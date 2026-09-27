//! `DownloadClient` for Transmission's RPC interface.

mod client;
mod wire;

pub use client::{TransmissionClient, TransmissionSettings};
