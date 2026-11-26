//! Errores que puede devolver el contrato.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    ValorInvalido = 1,      // menos de 1 XLM o más de 10 000
    CantidadInvalida = 2,   // 0 boletos
    SinLibres = 3,          // no hay tantos boletos con plata
    NumeroInvalido = 4,     // entrega repetida o salteada
    RecolectorInvalido = 5, // el propio contrato
    NadaParaCobrar = 6,     // el recolector no tiene boletos
    MedidorInvalido = 7,    // el propio contrato
}
