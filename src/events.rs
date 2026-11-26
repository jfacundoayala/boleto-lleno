//! Los avisos públicos del contrato. Cada cambio deja uno.

use soroban_sdk::{contractevent, Address};

/// El productor apartó la plata de `boletos`.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cargado {
    pub boletos: u32,
}

/// El medidor le dio `boletos` a un recolector. Se puede filtrar por recolector.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entregado {
    #[topic]
    pub recolector: Address,
    pub numero: u32,
    pub boletos: u32,
}

/// Un recolector cambió sus boletos por `monto`. Se puede filtrar por recolector.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cobrado {
    #[topic]
    pub recolector: Address,
    pub boletos: u32,
    pub monto: i128,
}

/// El productor recuperó la plata de `boletos` libres.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Retirado {
    pub boletos: u32,
}

/// El productor cambió al medidor.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MedidorCambiado {
    pub anterior: Address,
    pub medidor: Address,
}
