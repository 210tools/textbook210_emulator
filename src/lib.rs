#![warn(clippy::all, rust_2018_idioms)]
#![allow(refining_impl_trait)]

pub mod app;
pub mod emulator;
pub mod examples;
pub mod panes;
pub mod theme;
pub mod turing;

pub use app::EmulatorApp;
