use soroban_sdk::{contracttype, Address, Env};

use crate::constants::{TTL_ESTIRAR_HASTA, TTL_RENOVAR_SI_QUEDAN};
use crate::model::{Datos, Estado};

#[contracttype]
pub enum DataKey {
    Datos,            // instance
    Estado,           // instance
    Boletos(Address), // persistent, uno por recolector
}

pub fn leer_datos(env: &Env) -> Datos {
    env.storage().instance().get(&DataKey::Datos).unwrap()
}

pub fn guardar_datos(env: &Env, datos: &Datos) {
    let s = env.storage().instance();
    s.set(&DataKey::Datos, datos);
    s.extend_ttl(TTL_RENOVAR_SI_QUEDAN, TTL_ESTIRAR_HASTA);
}

pub fn leer_estado(env: &Env) -> Estado {
    env.storage().instance().get(&DataKey::Estado).unwrap()
}

pub fn guardar_estado(env: &Env, estado: &Estado) {
    let s = env.storage().instance();
    s.set(&DataKey::Estado, estado);
    s.extend_ttl(TTL_RENOVAR_SI_QUEDAN, TTL_ESTIRAR_HASTA);
}

pub fn leer_boletos(env: &Env, recolector: &Address) -> u32 {
    let clave = DataKey::Boletos(recolector.clone());
    env.storage().persistent().get(&clave).unwrap_or(0)
}

pub fn guardar_boletos(env: &Env, recolector: &Address, boletos: u32) {
    let clave = DataKey::Boletos(recolector.clone());
    let s = env.storage().persistent();
    s.set(&clave, &boletos);
    s.extend_ttl(&clave, TTL_RENOVAR_SI_QUEDAN, TTL_ESTIRAR_HASTA);
}

pub fn borrar_boletos(env: &Env, recolector: &Address) {
    env.storage().persistent().remove(&DataKey::Boletos(recolector.clone()));
}
