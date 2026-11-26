//! Boleto Lleno: un boleto de café que solo se entrega si su plata ya está apartada.

#![no_std]

mod constants;
mod contract;
mod errors;
mod events;
mod model;
mod storage;

#[cfg(test)]
mod test;

pub use contract::{BoletoLleno, BoletoLlenoClient, BoletoLlenoInterface};
pub use errors::Error;
pub use events::{Cargado, Cobrado, Entregado, MedidorCambiado, Retirado};
pub use model::{Datos, Estado};
