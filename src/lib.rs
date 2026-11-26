// BOLETO LLENO
// Un boleto de café que no se puede entregar vacío: solo se entrega si su plata
// ya está apartada, y el productor ya no la puede sacar.
//
//   1. cargar   - el productor aparta plata para N boletos
//   2. entregar - el medidor da boletos por el café medido
//   3. cobrar   - los boletos se cambian por plata para el recolector
//   4. retirar  - el productor recupera solo lo que no se entregó
//
// Además el productor puede cambiar al medidor. La moneda es siempre XLM.
//
// Archivos:
//   lib.rs      el contrato y sus funciones públicas
//   storage.rs  qué se guarda y cuánto dura (DataKey, get/set, TTL)
//   errors.rs   los errores, con número fijo
//   events.rs   los avisos públicos
//   types.rs    Datos y Estado
//   test.rs     los tests
#![no_std]
use soroban_sdk::{
    contract, contractimpl, contractmeta, panic_with_error, token, Address, Bytes, Env,
};

mod errors;
mod events;
mod storage;
mod types;

#[cfg(test)]
mod test;

pub use errors::Error;
pub use events::{Cargado, Cobrado, Entregado, MedidorCambiado, Retirado};
pub use storage::DataKey;
pub use types::{Datos, Estado};

#[cfg(test)]
use storage::DIA;

contractmeta!(
    key = "Description",
    val = "Boleto Lleno: un boleto de cafe que solo se entrega si su plata ya esta apartada"
);

const VALOR_MINIMO: i128 = 10_000_000; // 1 XLM: mínimo para activar una cuenta nueva, según CAP-73 (le queda de reserva)
const VALOR_MAXIMO: i128 = 100_000_000_000; // 10 000 XLM: tope para que las cuentas nunca se pasen
const XLM: [u8; 4] = [0, 0, 0, 0]; // así se escribe "XLM" (Asset::Native) en el formato de la red

#[contract]
pub struct BoletoLleno;

#[contractimpl]
impl BoletoLleno {
    /// Corre una sola vez, al desplegar: fija los roles y el valor. La moneda es XLM.
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

    /// 1. El productor aparta la plata de esa cantidad de boletos.
    pub fn cargar(env: Env, boletos: u32) -> Result<(), Error> {
        let datos = storage::datos(&env);
        datos.productor.require_auth(); // firma quien pone la plata
        if boletos == 0 {
            return Err(Error::CantidadInvalida);
        }

        let mut estado = storage::estado(&env);
        estado.libres += boletos;
        storage::guardar_estado(&env, &estado);
        mover(&env, &datos, &datos.productor, &env.current_contract_address(), boletos);
        Cargado { boletos }.publish(&env);
        Ok(())
    }

    /// 2. El medidor entrega boletos por el café medido. Solo si tienen plata.
    ///    El número va de a uno: repetir el envío no entrega dos veces.
    pub fn entregar(env: Env, numero: u32, recolector: Address, boletos: u32) -> Result<(), Error> {
        storage::datos(&env).medidor.require_auth(); // firma quien midió
        let mut estado = storage::estado(&env);
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
        let suyos = storage::boletos(&env, &recolector) + boletos;
        storage::guardar_boletos(&env, &recolector, suyos);

        Entregado { recolector, numero, boletos }.publish(&env);
        Ok(())
    }

    /// 3. Cambia por plata todos los boletos de un recolector. Devuelve cuánto cobró.
    ///    No pide firma: la plata solo puede ir al recolector, la pida quien la pida.
    pub fn cobrar(env: Env, recolector: Address) -> Result<i128, Error> {
        let boletos = storage::boletos(&env, &recolector);
        if boletos == 0 {
            return Err(Error::NadaParaCobrar);
        }

        // primero se borran los boletos y después sale la plata
        storage::borrar_boletos(&env, &recolector);
        let mut estado = storage::estado(&env);
        estado.entregados -= boletos;
        storage::guardar_estado(&env, &estado);

        let datos = storage::datos(&env);
        let monto = mover(&env, &datos, &env.current_contract_address(), &recolector, boletos);
        Cobrado { recolector, boletos, monto }.publish(&env);
        Ok(monto)
    }

    /// 4. El productor recupera la plata de boletos libres. Los entregados no se tocan.
    pub fn retirar(env: Env, boletos: u32) -> Result<(), Error> {
        let datos = storage::datos(&env);
        datos.productor.require_auth();
        let mut estado = storage::estado(&env);
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

    /// El productor cambia al medidor, por ejemplo si perdió o le robaron la clave.
    /// Los boletos ya entregados no cambian.
    pub fn cambiar_medidor(env: Env, medidor: Address) -> Result<(), Error> {
        let mut datos = storage::datos(&env);
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

    /// Cuántos boletos sin cobrar tiene un recolector. Todos tienen su plata.
    pub fn boletos(env: Env, recolector: Address) -> u32 {
        storage::boletos(&env, &recolector)
    }

    /// Cuántos boletos hay libres y entregados, y el número de la última entrega.
    pub fn estado(env: Env) -> Estado {
        storage::estado(&env)
    }

    /// Lo que se fijó al desplegar: quiénes, con qué moneda y cuánto vale un boleto.
    pub fn datos(env: Env) -> Datos {
        storage::datos(&env)
    }
}

// Mueve la plata de esa cantidad de boletos. Devuelve el monto.
fn mover(env: &Env, datos: &Datos, de: &Address, para: &Address, boletos: u32) -> i128 {
    let monto = datos.valor * i128::from(boletos);
    token::TokenClient::new(env, &datos.moneda).transfer(de, para, &monto);
    monto
}
