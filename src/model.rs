//! Los datos que guarda el contrato.

use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Datos {
    pub productor: Address,
    pub medidor: Address,
    pub moneda: Address, // XLM
    pub valor: i128,     // de un boleto, en stroops
}

// plata del contrato >= valor * (libres + entregados)
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Estado {
    pub libres: u32,     // con plata, sin dueño
    pub entregados: u32, // con plata, falta cobrarlos
    pub ultima: u32,     // número de la última entrega
}
