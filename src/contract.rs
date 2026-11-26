//! Las funciones públicas del contrato.
//!
//! La interfaz (`BoletoLlenoInterface`) dice qué hace cada función y quién firma.
//! Abajo está la implementación.

use soroban_sdk::{
    contract, contractimpl, contractmeta, panic_with_error, token, Address, Bytes, Env,
};

use crate::constants::{VALOR_MAXIMO, VALOR_MINIMO, XLM};
use crate::errors::Error;
use crate::events::{Cargado, Cobrado, Entregado, MedidorCambiado, Retirado};
use crate::model::{Datos, Estado};
use crate::storage;

contractmeta!(
    key = "Description",
    val = "Boleto Lleno: un boleto de cafe que solo se entrega si su plata ya esta apartada"
);

/// Lo que se puede hacer con el contrato.
///
/// Regla que no se rompe: la plata del contrato alcanza para todos los boletos,
/// los libres y los entregados.
pub trait BoletoLlenoInterface {
    /// 1. El productor aparta la plata de esa cantidad de boletos. Firma el productor.
    /// Falla con `CantidadInvalida` si son 0.
    fn cargar(env: Env, boletos: u32) -> Result<(), Error>;

    /// 2. El medidor entrega boletos por el café medido, solo si tienen plata.
    /// Firma el medidor. El número va de a uno: repetir el envío no entrega dos veces.
    /// Falla con `CantidadInvalida`, `NumeroInvalido`, `RecolectorInvalido` o `SinLibres`.
    fn entregar(env: Env, numero: u32, recolector: Address, boletos: u32) -> Result<(), Error>;

    /// 3. Cambia por plata todos los boletos de un recolector y devuelve cuánto cobró.
    /// No pide firma: la plata solo puede ir al recolector, la pida quien la pida.
    /// Falla con `NadaParaCobrar` si no tiene boletos.
    fn cobrar(env: Env, recolector: Address) -> Result<i128, Error>;

    /// 4. El productor recupera la plata de boletos libres. Los entregados no se tocan.
    /// Firma el productor. Falla con `CantidadInvalida` o `SinLibres`.
    fn retirar(env: Env, boletos: u32) -> Result<(), Error>;

    /// El productor cambia al medidor, por ejemplo si perdió o le robaron la clave.
    /// Firma el productor. Los boletos ya entregados no cambian.
    /// Falla con `MedidorInvalido` si el nuevo medidor es el propio contrato.
    fn cambiar_medidor(env: Env, medidor: Address) -> Result<(), Error>;

    /// Cuántos boletos sin cobrar tiene un recolector. Todos tienen su plata.
    fn boletos(env: Env, recolector: Address) -> u32;

    /// Cuántos boletos hay libres y entregados, y el número de la última entrega.
    fn estado(env: Env) -> Estado;

    /// Quiénes son el productor y el medidor, la moneda (XLM) y cuánto vale un boleto.
    fn datos(env: Env) -> Datos;
}

#[contract]
pub struct BoletoLleno;

#[contractimpl]
impl BoletoLleno {
    /// Corre una sola vez, al desplegar: fija los roles y el valor. La moneda es XLM.
    /// Falla con `ValorInvalido` (menos de 1 o más de 10 000 XLM) o `MedidorInvalido`.
    pub fn __constructor(env: Env, productor: Address, medidor: Address, valor: i128) {
        if !(VALOR_MINIMO..=VALOR_MAXIMO).contains(&valor) {
            panic_with_error!(&env, Error::ValorInvalido);
        }
        if medidor == env.current_contract_address() {
            panic_with_error!(&env, Error::MedidorInvalido);
        }
        // La dirección del contrato de XLM en esta red. Nadie la puede cambiar por otra moneda.
        let xlm = Bytes::from_array(&env, &XLM);
        let moneda = env.deployer().with_stellar_asset(xlm).deployed_address();
        storage::guardar_datos(&env, &Datos { productor, medidor, moneda, valor });
        storage::guardar_estado(&env, &Estado { libres: 0, entregados: 0, ultima: 0 });
    }
}

#[contractimpl]
impl BoletoLlenoInterface for BoletoLleno {
    fn cargar(env: Env, boletos: u32) -> Result<(), Error> {
        let datos = storage::leer_datos(&env);
        datos.productor.require_auth(); // firma quien pone la plata
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
        storage::leer_datos(&env).medidor.require_auth(); // firma quien midió
        let mut estado = storage::leer_estado(&env);
        if boletos == 0 {
            return Err(Error::CantidadInvalida);
        }
        if numero != estado.ultima + 1 {
            return Err(Error::NumeroInvalido);
        }
        if recolector == env.current_contract_address() {
            return Err(Error::RecolectorInvalido);
        }
        if boletos > estado.libres {
            return Err(Error::SinLibres); // acá está la idea: no hay boleto vacío
        }

        // dejan de estar libres y quedan a nombre del recolector
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
        let mut estado = storage::leer_estado(&env);
        if boletos == 0 {
            return Err(Error::CantidadInvalida);
        }
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

    // Consultas: solo leen, no piden firma

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

// Mueve la plata de esa cantidad de boletos. Devuelve el monto.
fn mover(env: &Env, datos: &Datos, de: &Address, para: &Address, boletos: u32) -> i128 {
    let monto = datos.valor * i128::from(boletos);
    token::TokenClient::new(env, &datos.moneda).transfer(de, para, &monto);
    monto
}
