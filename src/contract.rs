//! Funciones públicas del contrato.

use soroban_sdk::{
    contract, contractimpl, contractmeta, panic_with_error, token, Address, Bytes, Env,
};

use crate::constants::{ACTIVO_XLM, VALOR_MAXIMO, VALOR_MINIMO};
use crate::errors::Error;
use crate::events::{Cargado, Cobrado, Entregado, MedidorCambiado, Retirado};
use crate::model::{Datos, Estado};
use crate::storage;

contractmeta!(
    key = "Description",
    val = "Boleto Lleno: un boleto de cafe que solo se entrega si su plata ya esta apartada"
);

pub trait BoletoLlenoInterface {
    /// El productor aparta la plata de los boletos.
    fn cargar(env: Env, boletos: u32) -> Result<(), Error>;

    /// El medidor da boletos a un recolector, solo si hay libres.
    /// `numero` va de a uno, así una entrega repetida no cuenta dos veces.
    fn entregar(env: Env, numero: u32, recolector: Address, boletos: u32) -> Result<(), Error>;

    /// Paga los boletos del recolector. Lo puede pedir cualquiera.
    fn cobrar(env: Env, recolector: Address) -> Result<i128, Error>;

    /// El productor saca la plata de boletos libres.
    fn retirar(env: Env, boletos: u32) -> Result<(), Error>;

    /// El productor pone otro medidor (por ejemplo si perdió la clave).
    fn cambiar_medidor(env: Env, medidor: Address) -> Result<(), Error>;

    /// Boletos sin cobrar de un recolector.
    fn boletos(env: Env, recolector: Address) -> u32;

    /// Cuántos boletos hay libres y entregados.
    fn estado(env: Env) -> Estado;

    /// Productor, medidor, moneda y valor del boleto.
    fn datos(env: Env) -> Datos;
}

#[contract]
pub struct BoletoLleno;

#[contractimpl]
impl BoletoLleno {
    pub fn __constructor(env: Env, productor: Address, medidor: Address, valor: i128) {
        if !(VALOR_MINIMO..=VALOR_MAXIMO).contains(&valor) {
            panic_with_error!(&env, Error::ValorInvalido);
        }
        if medidor == env.current_contract_address() {
            panic_with_error!(&env, Error::MedidorInvalido);
        }
        // la moneda es siempre XLM
        let xlm = Bytes::from_array(&env, &ACTIVO_XLM);
        let moneda = env.deployer().with_stellar_asset(xlm).deployed_address();
        storage::guardar_datos(&env, &Datos { productor, medidor, moneda, valor });
        storage::guardar_estado(&env, &Estado { libres: 0, entregados: 0, ultima: 0 });
    }
}

#[contractimpl]
impl BoletoLlenoInterface for BoletoLleno {
    fn cargar(env: Env, boletos: u32) -> Result<(), Error> {
        let datos = storage::leer_datos(&env);
        datos.productor.require_auth();
        if boletos == 0 {
            return Err(Error::CantidadInvalida);
        }

        let mut estado = storage::leer_estado(&env);
        estado.libres += boletos;
        storage::guardar_estado(&env, &estado);
        mover(&env, &datos, &datos.productor, &env.current_contract_address(), boletos);
        Cargado { boletos }.publish(&env);
        Ok(())
    }

    fn entregar(env: Env, numero: u32, recolector: Address, boletos: u32) -> Result<(), Error> {
        storage::leer_datos(&env).medidor.require_auth();
        if boletos == 0 {
            return Err(Error::CantidadInvalida);
        }

        let mut estado = storage::leer_estado(&env);
        if numero != estado.ultima + 1 {
            return Err(Error::NumeroInvalido);
        }
        if recolector == env.current_contract_address() {
            return Err(Error::RecolectorInvalido);
        }
        if boletos > estado.libres {
            return Err(Error::SinLibres); // acá está la idea: no hay boleto vacío
        }

        estado.libres -= boletos;
        estado.entregados += boletos;
        estado.ultima = numero;
        storage::guardar_estado(&env, &estado);
        let suyos = storage::leer_boletos(&env, &recolector) + boletos;
        storage::guardar_boletos(&env, &recolector, suyos);

        Entregado { recolector, numero, boletos }.publish(&env);
        Ok(())
    }

    fn cobrar(env: Env, recolector: Address) -> Result<i128, Error> {
        let boletos = storage::leer_boletos(&env, &recolector);
        if boletos == 0 {
            return Err(Error::NadaParaCobrar);
        }

        // primero se borran los boletos y después sale la plata
        storage::borrar_boletos(&env, &recolector);
        let mut estado = storage::leer_estado(&env);
        estado.entregados -= boletos;
        storage::guardar_estado(&env, &estado);

        let datos = storage::leer_datos(&env);
        let monto = mover(&env, &datos, &env.current_contract_address(), &recolector, boletos);
        Cobrado { recolector, boletos, monto }.publish(&env);
        Ok(monto)
    }

    fn retirar(env: Env, boletos: u32) -> Result<(), Error> {
        let datos = storage::leer_datos(&env);
        datos.productor.require_auth();
        if boletos == 0 {
            return Err(Error::CantidadInvalida);
        }

        let mut estado = storage::leer_estado(&env);
        if boletos > estado.libres {
            return Err(Error::SinLibres);
        }

        estado.libres -= boletos;
        storage::guardar_estado(&env, &estado);
        mover(&env, &datos, &env.current_contract_address(), &datos.productor, boletos);
        Retirado { boletos }.publish(&env);
        Ok(())
    }

    fn cambiar_medidor(env: Env, medidor: Address) -> Result<(), Error> {
        let mut datos = storage::leer_datos(&env);
        datos.productor.require_auth();
        if medidor == env.current_contract_address() {
            return Err(Error::MedidorInvalido);
        }

        let anterior = datos.medidor;
        datos.medidor = medidor.clone();
        storage::guardar_datos(&env, &datos);
        MedidorCambiado { anterior, medidor }.publish(&env);
        Ok(())
    }

    // consultas

    fn boletos(env: Env, recolector: Address) -> u32 {
        storage::leer_boletos(&env, &recolector)
    }

    fn estado(env: Env) -> Estado {
        storage::leer_estado(&env)
    }

    fn datos(env: Env) -> Datos {
        storage::leer_datos(&env)
    }
}

fn mover(env: &Env, datos: &Datos, de: &Address, para: &Address, boletos: u32) -> i128 {
    let monto = datos.valor * i128::from(boletos);
    token::TokenClient::new(env, &datos.moneda).transfer(de, para, &monto);
    monto
}
