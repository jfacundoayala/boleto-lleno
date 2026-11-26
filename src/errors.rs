use soroban_sdk::contracterror;

// Los errores, con número fijo para reconocerlos desde afuera.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    ValorInvalido = 1,      // un boleto vale entre 1 y 10 000 XLM
    CantidadInvalida = 2,   // 0 boletos
    SinLibres = 3,          // no hay tantos boletos con plata
    NumeroInvalido = 4,     // esa entrega ya se hizo, o se saltó una
    RecolectorInvalido = 5, // el contrato no puede ser recolector: la plata quedaría trabada
    NadaParaCobrar = 6,
    MedidorInvalido = 7, // el contrato no puede ser medidor: nadie podría entregar
}
