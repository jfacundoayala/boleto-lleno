use soroban_sdk::{contracttype, Address};

// Se fija al desplegar. Solo el medidor se puede cambiar, y lo hace el productor.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Datos {
    pub productor: Address, // pone la plata
    pub medidor: Address,   // mide el café y entrega los boletos
    pub moneda: Address,    // siempre XLM: la calcula el contrato, no se elige
    pub valor: i128,        // cuánta plata vale un boleto
}

// Regla que no se rompe: plata del contrato >= valor x (libres + entregados).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Estado {
    pub libres: u32,     // con plata, todavía de nadie
    pub entregados: u32, // con plata, ya son de un recolector y falta cobrarlos
    pub ultima: u32,     // número de la última entrega (0 = ninguna)
}
