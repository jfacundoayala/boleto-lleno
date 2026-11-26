//! Los datos que guarda el contrato.

use soroban_sdk::{contracttype, Address};

/// Se fija al desplegar. Solo el medidor se puede cambiar, y lo hace el productor.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Datos {
    /// Pone la plata.
    pub productor: Address,
    /// Mide el café y entrega los boletos.
    pub medidor: Address,
    /// Siempre XLM: la calcula el contrato, no se elige.
    pub moneda: Address,
    /// Cuánta plata vale un boleto, en stroops (1 XLM = 10 000 000).
    pub valor: i128,
}

/// Cuántos boletos hay. Regla que no se rompe:
/// plata del contrato >= valor x (libres + entregados).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Estado {
    /// Con plata, todavía de nadie.
    pub libres: u32,
    /// Con plata, ya son de un recolector y falta cobrarlos.
    pub entregados: u32,
    /// Número de la última entrega (0 = ninguna).
    pub ultima: u32,
}
