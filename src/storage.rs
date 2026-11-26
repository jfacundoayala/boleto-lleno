use soroban_sdk::{contracttype, Address, Env};

use crate::types::{Datos, Estado};

pub const DIA: u32 = 17_280; // ledgers en un día (uno cada ~5 segundos)

// Si a un dato le quedan menos de 30 días de vida (TTL), se estira a unos 120.
// Si igual se archiva no se pierde: se restaura al usarlo.
const RENOVAR_SI_QUEDAN: u32 = 30 * DIA;
const ESTIRAR_HASTA: u32 = 120 * DIA;

// Las claves de lo que el contrato guarda, igual que en el contador.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Datos,            // quiénes, la moneda y el valor
    Estado,           // cuántos boletos hay
    Boletos(Address), // cuántos boletos sin cobrar tiene cada recolector
}

pub fn datos(env: &Env) -> Datos {
    env.storage().instance().get(&DataKey::Datos).unwrap()
}

pub fn guardar_datos(env: &Env, datos: &Datos) {
    let s = env.storage().instance();
    s.set(&DataKey::Datos, datos);
    s.extend_ttl(RENOVAR_SI_QUEDAN, ESTIRAR_HASTA);
}

pub fn estado(env: &Env) -> Estado {
    env.storage().instance().get(&DataKey::Estado).unwrap()
}

pub fn guardar_estado(env: &Env, estado: &Estado) {
    let s = env.storage().instance();
    s.set(&DataKey::Estado, estado);
    s.extend_ttl(RENOVAR_SI_QUEDAN, ESTIRAR_HASTA);
}

pub fn boletos(env: &Env, recolector: &Address) -> u32 {
    let clave = DataKey::Boletos(recolector.clone());
    env.storage().persistent().get(&clave).unwrap_or(0)
}

pub fn guardar_boletos(env: &Env, recolector: &Address, boletos: u32) {
    let clave = DataKey::Boletos(recolector.clone());
    let s = env.storage().persistent();
    s.set(&clave, &boletos);
    s.extend_ttl(&clave, RENOVAR_SI_QUEDAN, ESTIRAR_HASTA);
}

pub fn borrar_boletos(env: &Env, recolector: &Address) {
    env.storage().persistent().remove(&DataKey::Boletos(recolector.clone()));
}
