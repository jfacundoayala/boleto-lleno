// Se corren con:  cargo test
extern crate std;

use super::*;
use soroban_sdk::testutils::storage::{Instance as _, Persistent as _};
use soroban_sdk::testutils::{
    Address as _, AuthorizedFunction, Events as _, Ledger as _, MockAuth, MockAuthInvoke,
};
use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::{Event as _, IntoVal, Symbol, Val, Vec};

const VALOR: i128 = 20_000_000; // un boleto vale 2 XLM

struct Prueba {
    env: Env,
    contrato: BoletoLlenoClient<'static>,
    token: TokenClient<'static>,
    productor: Address,
    medidor: Address,
    rosa: Address, // recolectora
    luis: Address, // recolector
}

// Arma todo: una moneda de prueba, el contrato y el productor con 100 boletos de plata.
fn setup() -> Prueba {
    let env = Env::default();
    env.mock_all_auths(); // todos firman

    let emisor = Address::generate(&env);
    let moneda = env.register_stellar_asset_contract_v2(emisor).address();
    let productor = Address::generate(&env);
    let medidor = Address::generate(&env);
    StellarAssetClient::new(&env, &moneda).mint(&productor, &(100 * VALOR));
    let id = env.register(BoletoLleno, (&productor, &medidor, &moneda, VALOR));

    Prueba {
        contrato: BoletoLlenoClient::new(&env, &id),
        token: TokenClient::new(&env, &moneda),
        rosa: Address::generate(&env),
        luis: Address::generate(&env),
        productor,
        medidor,
        env,
    }
}

impl Prueba {
    // La regla que no se rompe: hay plata para todos los boletos.
    fn lleno(&self) -> bool {
        let e = self.contrato.estado();
        let plata = self.token.balance(&self.contrato.address);
        plata >= VALOR * i128::from(e.libres + e.entregados)
    }

    // A partir de acá solo vale la firma de `quien` para esta llamada exacta.
    fn solo_firma(&self, quien: &Address, funcion: &'static str, args: Vec<Val>) {
        self.env.mock_auths(&[MockAuth {
            address: quien,
            invoke: &MockAuthInvoke {
                contract: &self.contrato.address,
                fn_name: funcion,
                args,
                sub_invokes: &[],
            },
        }]);
    }

    // Quién firmó la última llamada y qué función.
    fn firmo(&self, quien: &Address, funcion: &str, args: Vec<Val>) -> bool {
        let firmas = self.env.auths();
        firmas.len() == 1
            && firmas[0].0 == *quien
            && firmas[0].1.function
                == AuthorizedFunction::Contract((
                    self.contrato.address.clone(),
                    Symbol::new(&self.env, funcion),
                    args,
                ))
    }
}

// 1. el camino feliz

#[test]
fn cargar_entregar_cobrar_y_retirar_lo_que_sobra() {
    let p = setup();
    p.contrato.cargar(&5);
    p.contrato.entregar(&1, &p.rosa, &3);
    assert_eq!(p.contrato.boletos(&p.rosa), 3);
    assert_eq!(p.contrato.estado(), Estado { libres: 2, entregados: 3, ultima: 1 });

    assert_eq!(p.contrato.cobrar(&p.rosa), 3 * VALOR);
    p.contrato.retirar(&2);

    assert_eq!(p.token.balance(&p.rosa), 3 * VALOR);
    assert_eq!(p.token.balance(&p.productor), 97 * VALOR);
    assert_eq!(p.token.balance(&p.contrato.address), 0);
    assert_eq!(p.contrato.estado(), Estado { libres: 0, entregados: 0, ultima: 1 });
}

// Varias entregas se juntan y se cobran en una sola transferencia.
#[test]
fn las_entregas_se_juntan() {
    let p = setup();
    p.contrato.cargar(&10);
    p.contrato.entregar(&1, &p.rosa, &2);
    p.contrato.entregar(&2, &p.luis, &1);
    p.contrato.entregar(&3, &p.rosa, &4);
    assert_eq!(p.contrato.cobrar(&p.rosa), 6 * VALOR);
    assert_eq!(p.contrato.boletos(&p.luis), 1); // lo de Luis no se toca
}

// 2. no hay boleto vacío

#[test]
fn sin_plata_no_hay_boleto() {
    let p = setup();
    let r = p.contrato.try_entregar(&1, &p.rosa, &1);
    assert_eq!(r, Err(Ok(Error::SinLibres)));
    p.contrato.cargar(&2);
    let r = p.contrato.try_entregar(&1, &p.rosa, &3); // uno más de los que hay
    assert_eq!(r, Err(Ok(Error::SinLibres)));
    assert_eq!(p.contrato.boletos(&p.rosa), 0);
    p.contrato.entregar(&1, &p.rosa, &2); // justo los que hay sí se puede
    assert_eq!(p.contrato.boletos(&p.rosa), 2);
}

#[test]
fn lo_entregado_no_se_puede_retirar() {
    let p = setup();
    p.contrato.cargar(&5);
    p.contrato.entregar(&1, &p.rosa, &3);
    assert_eq!(p.contrato.try_retirar(&3), Err(Ok(Error::SinLibres))); // solo hay 2 libres
    p.contrato.retirar(&2);
    assert_eq!(p.contrato.try_retirar(&1), Err(Ok(Error::SinLibres)));
    assert_eq!(p.contrato.cobrar(&p.rosa), 3 * VALOR); // Rosa cobra igual
}

// Si al productor no le alcanza, no aparece ningún boleto.
#[test]
fn si_la_plata_no_alcanza_no_se_carga_nada() {
    let p = setup();
    assert!(p.contrato.try_cargar(&101).is_err());
    assert_eq!(p.contrato.estado().libres, 0);
    assert_eq!(p.token.balance(&p.contrato.address), 0);
}

// Plata mandada por fuera no crea boletos: no se entregan sin cargarlos.
#[test]
fn la_plata_de_afuera_no_crea_boletos() {
    let p = setup();
    p.token.transfer(&p.productor, &p.contrato.address, &(5 * VALOR));
    let r = p.contrato.try_entregar(&1, &p.rosa, &1);
    assert_eq!(r, Err(Ok(Error::SinLibres)));
}

// Una secuencia larga, revisando la regla después de cada paso.
#[test]
fn siempre_hay_plata_para_todos_los_boletos() {
    let p = setup();
    let (mut n, mut total) = (0, 0);
    for vuelta in 1..=5u32 {
        p.contrato.cargar(&(vuelta * 3));
        assert!(p.lleno());
        for quien in [&p.rosa, &p.luis] {
            n += 1;
            p.contrato.entregar(&n, quien, &vuelta);
            assert!(p.lleno());
        }
        p.contrato.retirar(&vuelta);
        assert!(p.lleno());
        if vuelta % 2 == 0 {
            total += p.contrato.cobrar(&p.rosa);
            assert!(p.lleno());
        }
    }
    total += p.contrato.cobrar(&p.rosa) + p.contrato.cobrar(&p.luis);
    assert_eq!(total, 30 * VALOR); // (1+2+3+4+5) boletos a cada uno
    assert_eq!(p.token.balance(&p.contrato.address), 0);
    assert_eq!(p.token.balance(&p.productor), 70 * VALOR);
}

// 3. cobrar

#[test]
fn no_se_cobra_dos_veces() {
    let p = setup();
    p.contrato.cargar(&5);
    p.contrato.entregar(&1, &p.rosa, &2);
    p.contrato.cobrar(&p.rosa);
    assert_eq!(p.contrato.try_cobrar(&p.rosa), Err(Ok(Error::NadaParaCobrar)));
    assert_eq!(p.contrato.try_cobrar(&p.luis), Err(Ok(Error::NadaParaCobrar)));
    assert_eq!(p.token.balance(&p.rosa), 2 * VALOR);
}

// Sin ninguna firma, la plata llega igual a Rosa y a nadie más.
#[test]
fn cualquiera_puede_pedir_el_cobro_pero_le_llega_al_recolector() {
    let p = setup();
    p.contrato.cargar(&5);
    p.contrato.entregar(&1, &p.rosa, &2);
    p.env.mock_auths(&[]); // desde acá no firma nadie
    assert_eq!(p.contrato.cobrar(&p.rosa), 2 * VALOR);
    assert_eq!(p.token.balance(&p.rosa), 2 * VALOR);
    assert_eq!(p.token.balance(&p.contrato.address), 3 * VALOR); // y no salió nada más
}

// 4. lo que no se puede

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn un_boleto_de_menos_de_1_xlm_falla() {
    let p = setup();
    let menos = 9_999_999i128;
    p.env.register(BoletoLleno, (&p.productor, &p.medidor, &p.token.address, menos));
}

#[test]
fn un_boleto_de_1_xlm_justo_sirve() {
    let p = setup();
    let justo = 10_000_000i128;
    let id = p.env.register(BoletoLleno, (&p.productor, &p.medidor, &p.token.address, justo));
    assert_eq!(BoletoLlenoClient::new(&p.env, &id).datos().valor, justo);
}

#[test]
fn cero_boletos_falla() {
    let p = setup();
    p.contrato.cargar(&5);
    assert_eq!(p.contrato.try_cargar(&0), Err(Ok(Error::CantidadInvalida)));
    assert_eq!(p.contrato.try_retirar(&0), Err(Ok(Error::CantidadInvalida)));
    let r = p.contrato.try_entregar(&1, &p.rosa, &0);
    assert_eq!(r, Err(Ok(Error::CantidadInvalida)));
}

// Se cortó la señal y el medidor mandó dos veces la misma entrega: vale una.
#[test]
fn la_misma_entrega_no_cuenta_dos_veces() {
    let p = setup();
    p.contrato.cargar(&10);
    p.contrato.entregar(&1, &p.rosa, &3);
    let r = p.contrato.try_entregar(&1, &p.rosa, &3); // repetida
    assert_eq!(r, Err(Ok(Error::NumeroInvalido)));
    let r = p.contrato.try_entregar(&3, &p.rosa, &3); // se saltó la 2
    assert_eq!(r, Err(Ok(Error::NumeroInvalido)));
    assert_eq!(p.contrato.boletos(&p.rosa), 3);
}

// Los números son parte del trato con quien usa el contrato: no pueden cambiar.
#[test]
fn los_errores_tienen_numero_fijo() {
    assert_eq!(Error::ValorInvalido as u32, 1);
    assert_eq!(Error::CantidadInvalida as u32, 2);
    assert_eq!(Error::SinLibres as u32, 3);
    assert_eq!(Error::NumeroInvalido as u32, 4);
    assert_eq!(Error::RecolectorInvalido as u32, 5);
    assert_eq!(Error::NadaParaCobrar as u32, 6);
}

#[test]
fn el_contrato_no_puede_ser_recolector() {
    let p = setup();
    p.contrato.cargar(&5);
    let r = p.contrato.try_entregar(&1, &p.contrato.address, &1);
    assert_eq!(r, Err(Ok(Error::RecolectorInvalido)));
}

// 5. las firmas

#[test]
fn cada_uno_firma_lo_suyo() {
    let p = setup();
    p.contrato.cargar(&5);
    assert!(p.firmo(&p.productor, "cargar", (5u32,).into_val(&p.env)));
    p.contrato.entregar(&1, &p.rosa, &2);
    assert!(p.firmo(&p.medidor, "entregar", (1u32, &p.rosa, 2u32).into_val(&p.env)));
    p.contrato.retirar(&1);
    assert!(p.firmo(&p.productor, "retirar", (1u32,).into_val(&p.env)));
}

#[test]
fn el_productor_no_puede_entregar() {
    let p = setup();
    p.contrato.cargar(&5);
    p.solo_firma(&p.productor, "entregar", (1u32, &p.rosa, 2u32).into_val(&p.env));
    assert!(p.contrato.try_entregar(&1, &p.rosa, &2).is_err());
    assert_eq!(p.contrato.boletos(&p.rosa), 0);
}

#[test]
fn el_medidor_no_puede_retirar() {
    let p = setup();
    p.contrato.cargar(&5);
    p.solo_firma(&p.medidor, "retirar", (2u32,).into_val(&p.env));
    assert!(p.contrato.try_retirar(&2).is_err());
    assert_eq!(p.contrato.estado().libres, 5);
}

// En una finca pequeña el productor y el medidor pueden ser la misma cuenta.
#[test]
fn con_una_sola_cuenta_lo_entregado_sigue_protegido() {
    let p = setup();
    let id = p.env.register(BoletoLleno, (&p.productor, &p.productor, &p.token.address, VALOR));
    let contrato = BoletoLlenoClient::new(&p.env, &id);
    contrato.cargar(&5);
    contrato.entregar(&1, &p.rosa, &3);
    assert_eq!(contrato.try_retirar(&3), Err(Ok(Error::SinLibres)));
    assert_eq!(contrato.cobrar(&p.rosa), 3 * VALOR);
}

// 6. lo que queda a la vista

#[test]
fn se_ven_los_datos() {
    let p = setup();
    let esperado = Datos {
        productor: p.productor.clone(),
        medidor: p.medidor.clone(),
        moneda: p.token.address.clone(),
        valor: VALOR,
    };
    assert_eq!(p.contrato.datos(), esperado);
}

#[test]
fn cada_paso_avisa_con_un_evento() {
    let p = setup();
    let id = &p.contrato.address;
    let avisos = || p.env.events().all().filter_by_contract(id);

    p.contrato.cargar(&5);
    assert_eq!(avisos(), std::vec![Cargado { boletos: 5 }.to_xdr(&p.env, id)]);

    p.contrato.entregar(&1, &p.rosa, &2);
    let e = Entregado { recolector: p.rosa.clone(), numero: 1, boletos: 2 };
    assert_eq!(e.topics(&p.env).len(), 2); // nombre y recolector: se puede filtrar por él
    assert_eq!(avisos(), std::vec![e.to_xdr(&p.env, id)]);

    p.contrato.cobrar(&p.rosa);
    let c = Cobrado { recolector: p.rosa.clone(), boletos: 2, monto: 2 * VALOR };
    assert_eq!(c.topics(&p.env).len(), 2);
    assert_eq!(avisos(), std::vec![c.to_xdr(&p.env, id)]);

    p.contrato.retirar(&3);
    assert_eq!(avisos(), std::vec![Retirado { boletos: 3 }.to_xdr(&p.env, id)]);
}

// La primera entrega deja 2 073 600 ledgers de vida (120 días). Cuando quedan menos
// de 30, otra entrega la vuelve a estirar.
#[test]
fn renueva_el_ttl() {
    let p = setup();
    p.contrato.cargar(&5);
    let vida = || {
        p.env.as_contract(&p.contrato.address, || {
            let s = p.env.storage();
            let clave = DataKey::Boletos(p.rosa.clone());
            (s.instance().get_ttl(), s.persistent().get_ttl(&clave))
        })
    };
    p.contrato.entregar(&1, &p.rosa, &2);
    assert_eq!(vida(), (2_073_600, 2_073_600));

    p.env.ledger().with_mut(|l| l.sequence_number += 91 * DIA); // pasan 91 días
    p.contrato.entregar(&2, &p.rosa, &1);
    assert_eq!(vida(), (2_073_600, 2_073_600));
}
