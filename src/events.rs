use soroban_sdk::{contractevent, Address};

// Los avisos públicos. El topic sirve para filtrar por recolector.

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cargado {
    pub boletos: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entregado {
    #[topic]
    pub recolector: Address,
    pub numero: u32,
    pub boletos: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cobrado {
    #[topic]
    pub recolector: Address,
    pub boletos: u32,
    pub monto: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Retirado {
    pub boletos: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MedidorCambiado {
    pub anterior: Address,
    pub medidor: Address,
}
