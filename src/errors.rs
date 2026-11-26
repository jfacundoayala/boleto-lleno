//! Los errores del contrato.

use soroban_sdk::contracterror;

/// Cada error tiene un número fijo para reconocerlo desde afuera. No se renumeran.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    /// El boleto vale menos de 1 XLM o más de 10 000 XLM.
    ValorInvalido = 1,
    /// Se pidieron 0 boletos.
    CantidadInvalida = 2,
    /// No hay tantos boletos libres con plata.
    SinLibres = 3,
    /// Esa entrega ya se hizo, o se saltó una.
    NumeroInvalido = 4,
    /// El recolector es el propio contrato: la plata quedaría trabada.
    RecolectorInvalido = 5,
    /// Ese recolector no tiene boletos.
    NadaParaCobrar = 6,
    /// El medidor es el propio contrato: nadie podría entregar.
    MedidorInvalido = 7,
}
