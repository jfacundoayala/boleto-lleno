//! Boleto Lleno
//!
//! Un boleto de café que no se puede entregar vacío: solo se entrega si su plata
//! ya está apartada, y el productor ya no la puede sacar.
//!
//! 1. `cargar`: el productor aparta plata para N boletos.
//! 2. `entregar`: el medidor da boletos por el café medido.
//! 3. `cobrar`: los boletos se cambian por plata para el recolector.
//! 4. `retirar`: el productor recupera solo lo que no se entregó.
//!
//! Además el productor puede cambiar al medidor. La moneda es siempre XLM.

#![no_std]

mod constants;
pub mod contract;
pub mod errors;
mod events;
pub mod model;
mod storage;

#[cfg(test)]
mod test;

pub use contract::{BoletoLleno, BoletoLlenoClient, BoletoLlenoInterface};
pub use errors::Error;
pub use events::{Cargado, Cobrado, Entregado, MedidorCambiado, Retirado};
pub use model::{Datos, Estado};

#[cfg(test)]
use {constants::DIA, storage::DataKey};
