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
#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contractmeta, contracttype,
    panic_with_error, token, Address, Bytes, Env,
};

contractmeta!(
    key = "Description",
    val = "Boleto Lleno: un boleto de cafe que solo se entrega si su plata ya esta apartada"
);

const VALOR_MINIMO: i128 = 10_000_000; // 1 XLM: mínimo para activar una cuenta nueva, según CAP-73 (le queda de reserva)
const DIA: u32 = 17_280; // ledgers en un día (uno cada ~5 segundos)
const XLM: [u8; 4] = [0, 0, 0, 0]; // así se escribe "XLM" (Asset::Native) en el formato de la red

// Las claves de lo que el contrato guarda, igual que en el contador.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Datos,            // lo que se fija al desplegar
    Estado,           // cuántos boletos hay
    Boletos(Address), // cuántos boletos sin cobrar tiene cada recolector
}

// Se fija al desplegar. No hay función para cambiarlo.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Datos {
    pub productor: Address, // pone la plata
    pub medidor: Address,   // mide el café y entrega los boletos (el productor lo puede cambiar)
    pub moneda: Address,    // siempre XLM: la calcula el contrato, no se elige
    pub valor: i128,        // cuánta plata vale un boleto
}

// Regla que no se rompe (con XLM): plata del contrato >= valor x (libres + entregados).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Estado {
    pub libres: u32,     // con plata, todavía de nadie
    pub entregados: u32, // con plata, ya son de un recolector y falta cobrarlos
    pub ultima: u32,     // número de la última entrega (0 = ninguna)
}

// Los errores, con número fijo para reconocerlos desde afuera.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    ValorInvalido = 1,      // un boleto tiene que valer al menos 1 XLM
    CantidadInvalida = 2,   // 0 boletos
    SinLibres = 3,          // no hay tantos boletos con plata
    NumeroInvalido = 4,     // esa entrega ya se hizo, o se saltó una
    RecolectorInvalido = 5, // el contrato no puede ser recolector: la plata quedaría trabada
    NadaParaCobrar = 6,
}

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
    pub medidor: Address,
}

#[contract]
pub struct BoletoLleno;

#[contractimpl]
impl BoletoLleno {
    /// Corre una sola vez, al desplegar: fija los roles y el valor. La moneda es XLM.
    pub fn __constructor(env: Env, productor: Address, medidor: Address, valor: i128) {
        if valor < VALOR_MINIMO {
            panic_with_error!(&env, Error::ValorInvalido);
        }
        // La dirección del contrato de XLM en esta red. Nadie la puede cambiar por otra moneda.
        let xlm = Bytes::from_array(&env, &XLM);
        let moneda = env.deployer().with_stellar_asset(xlm).deployed_address();
        let datos = Datos { productor, medidor, moneda, valor };
        env.storage().instance().set(&DataKey::Datos, &datos);
        guardar(&env, &Estado { libres: 0, entregados: 0, ultima: 0 });
    }

    /// 1. El productor aparta la plata de esa cantidad de boletos.
    pub fn cargar(env: Env, boletos: u32) -> Result<(), Error> {
        let datos = Self::datos(env.clone());
        datos.productor.require_auth(); // firma quien pone la plata
        if boletos == 0 {
            return Err(Error::CantidadInvalida);
        }

        let mut estado = Self::estado(env.clone());
        estado.libres += boletos;
        guardar(&env, &estado);
        mover(&env, &datos, &datos.productor, &env.current_contract_address(), boletos);
        Cargado { boletos }.publish(&env);
        Ok(())
    }

    /// 2. El medidor entrega boletos por el café medido. Solo si tienen plata.
    ///    El número va de a uno: repetir el envío no entrega dos veces.
    pub fn entregar(env: Env, numero: u32, recolector: Address, boletos: u32) -> Result<(), Error> {
        Self::datos(env.clone()).medidor.require_auth(); // firma quien midió
        let mut estado = Self::estado(env.clone());
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
        guardar(&env, &estado);

        let suyos = Self::boletos(env.clone(), recolector.clone()) + boletos;
        let clave = DataKey::Boletos(recolector.clone());
        env.storage().persistent().set(&clave, &suyos);
        env.storage().persistent().extend_ttl(&clave, 30 * DIA, 120 * DIA);

        Entregado { recolector, numero, boletos }.publish(&env);
        Ok(())
    }

    /// 3. Cambia por plata todos los boletos de un recolector. Devuelve cuánto cobró.
    ///    No pide firma: la plata solo puede ir al recolector, la pida quien la pida.
    pub fn cobrar(env: Env, recolector: Address) -> Result<i128, Error> {
        let boletos = Self::boletos(env.clone(), recolector.clone());
        if boletos == 0 {
            return Err(Error::NadaParaCobrar);
        }

        // primero se borran los boletos y después sale la plata
        env.storage().persistent().remove(&DataKey::Boletos(recolector.clone()));
        let mut estado = Self::estado(env.clone());
        estado.entregados -= boletos;
        guardar(&env, &estado);

        let datos = Self::datos(env.clone());
        let monto = mover(&env, &datos, &env.current_contract_address(), &recolector, boletos);
        Cobrado { recolector, boletos, monto }.publish(&env);
        Ok(monto)
    }

    /// 4. El productor recupera la plata de boletos libres. Los entregados no se tocan.
    pub fn retirar(env: Env, boletos: u32) -> Result<(), Error> {
        let datos = Self::datos(env.clone());
        datos.productor.require_auth();
        let mut estado = Self::estado(env.clone());
        if boletos == 0 {
            return Err(Error::CantidadInvalida);
        }
        if boletos > estado.libres {
            return Err(Error::SinLibres);
        }

        estado.libres -= boletos;
        guardar(&env, &estado);
        mover(&env, &datos, &env.current_contract_address(), &datos.productor, boletos);
        Retirado { boletos }.publish(&env);
        Ok(())
    }

    /// El productor cambia al medidor, por ejemplo si perdió o le robaron la clave.
    /// Los boletos ya entregados no cambian.
    pub fn cambiar_medidor(env: Env, medidor: Address) {
        let mut datos = Self::datos(env.clone());
        datos.productor.require_auth();
        datos.medidor = medidor.clone();
        let s = env.storage().instance();
        s.set(&DataKey::Datos, &datos);
        s.extend_ttl(30 * DIA, 120 * DIA);
        MedidorCambiado { medidor }.publish(&env);
    }

    // Consultas: solo leen, no piden firma

    /// Cuántos boletos sin cobrar tiene un recolector. Todos tienen su plata.
    pub fn boletos(env: Env, recolector: Address) -> u32 {
        env.storage().persistent().get(&DataKey::Boletos(recolector)).unwrap_or(0)
    }

    /// Cuántos boletos hay libres y entregados, y el número de la última entrega.
    pub fn estado(env: Env) -> Estado {
        env.storage().instance().get(&DataKey::Estado).unwrap()
    }

    /// Lo que se fijó al desplegar: quiénes, con qué moneda y cuánto vale un boleto.
    pub fn datos(env: Env) -> Datos {
        env.storage().instance().get(&DataKey::Datos).unwrap()
    }
}

// Ayudantes (no son funciones públicas del contrato)

// Mueve la plata de esa cantidad de boletos. Devuelve el monto.
fn mover(env: &Env, datos: &Datos, de: &Address, para: &Address, boletos: u32) -> i128 {
    let monto = datos.valor * i128::from(boletos);
    token::TokenClient::new(env, &datos.moneda).transfer(de, para, &monto);
    monto
}

// Guarda el estado y renueva la vida (TTL) del contrato: si le quedan menos de
// 30 días, la estira a unos 120. Si se archiva no se pierde: se restaura al usarlo.
fn guardar(env: &Env, estado: &Estado) {
    let s = env.storage().instance();
    s.set(&DataKey::Estado, estado);
    s.extend_ttl(30 * DIA, 120 * DIA);
}

#[cfg(test)]
mod test;
