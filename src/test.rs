// Se corren con:  cargo test
extern crate std;

use super::*;
use soroban_sdk::testutils::storage::{Instance as _, Persistent as _};
use soroban_sdk::testutils::{
    Address as _, AuthorizedFunction, Events as _, Ledger as _, MockAuth, MockAuthInvoke,
};
use soroban_sdk::token::TokenClient;
use soroban_sdk::{xdr, Event as _, IntoVal, Symbol, TryFromVal, TryIntoVal, Val, Vec};
use std::rc::Rc;

const VALOR: i128 = 20_000_000; // un boleto vale 2 XLM
const PLATA_INICIAL: i128 = 100 * VALOR; // el productor arranca con 100 boletos de plata

struct Prueba {
    env: Env,
    contrato: BoletoLlenoClient<'static>,
    xlm: TokenClient<'static>,
    productor: Address,
    medidor: Address,
    rosa: Address, // recolectora
    luis: Address, // recolector
}

// El contrato de XLM de la red de prueba. Es el mismo que el contrato calcula solo.
fn crear_xlm(env: &Env) -> Address {
    let crear = xdr::HostFunction::CreateContract(xdr::CreateContractArgs {
        contract_id_preimage: xdr::ContractIdPreimage::Asset(xdr::Asset::Native),
        executable: xdr::ContractExecutable::StellarAsset,
    });
    env.host().invoke_function(crear).unwrap().try_into_val(env).unwrap()
}

// XLM no se puede "imprimir" como una moneda de prueba, así que se crea una cuenta
// con saldo directo en el ledger de prueba.
fn cuenta_con_xlm(env: &Env, semilla: u8, stroops: i128) -> Address {
    let id = xdr::AccountId(xdr::PublicKey::PublicKeyTypeEd25519(xdr::Uint256([semilla; 32])));
    let clave = Rc::new(xdr::LedgerKey::Account(xdr::LedgerKeyAccount { account_id: id.clone() }));
    let cuenta = Rc::new(xdr::LedgerEntry {
        data: xdr::LedgerEntryData::Account(xdr::AccountEntry {
            account_id: id.clone(),
            balance: stroops as i64,
            flags: 0,
            home_domain: Default::default(),
            inflation_dest: None,
            num_sub_entries: 0,
            seq_num: xdr::SequenceNumber(0),
            thresholds: xdr::Thresholds([1; 4]),
            signers: xdr::VecM::default(),
            ext: xdr::AccountEntryExt::V0,
        }),
        last_modified_ledger_seq: 0,
        ext: xdr::LedgerEntryExt::V0,
    });
    env.host().add_ledger_entry(&clave, &cuenta, None).unwrap();
    Address::try_from_val(env, &xdr::ScAddress::Account(id)).unwrap()
}

// Arma todo: XLM, el contrato y el productor con 100 boletos de plata.
fn setup() -> Prueba {
    let env = Env::default();
    env.mock_all_auths(); // todos firman

    let xlm = crear_xlm(&env);
    // La plata sale de una cuenta común y pasa al productor. El productor de la prueba
    // es una dirección generada, así se puede probar quién firma y quién no.
    let banco = cuenta_con_xlm(&env, 1, PLATA_INICIAL);
    let productor = Address::generate(&env);
    TokenClient::new(&env, &xlm).transfer(&banco, &productor, &PLATA_INICIAL);
    let medidor = Address::generate(&env);
    let id = env.register(BoletoLleno, (&productor, &medidor, VALOR));

    Prueba {
        contrato: BoletoLlenoClient::new(&env, &id),
        xlm: TokenClient::new(&env, &xlm),
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
        let plata = self.xlm.balance(&self.contrato.address);
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

    assert_eq!(p.xlm.balance(&p.rosa), 3 * VALOR);
    assert_eq!(p.xlm.balance(&p.productor), 97 * VALOR);
    assert_eq!(p.xlm.balance(&p.contrato.address), 0);
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
    assert_eq!(p.xlm.balance(&p.contrato.address), 0);
}

// Plata mandada por fuera no crea boletos: no se entregan sin cargarlos.
#[test]
fn la_plata_de_afuera_no_crea_boletos() {
    let p = setup();
    p.xlm.transfer(&p.productor, &p.contrato.address, &(5 * VALOR));
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
    assert_eq!(p.xlm.balance(&p.contrato.address), 0);
    assert_eq!(p.xlm.balance(&p.productor), 70 * VALOR);
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
    assert_eq!(p.xlm.balance(&p.rosa), 2 * VALOR);
}

// Como en la red de verdad: Rosa no tiene cuenta y hay reserva mínima (0,5 XLM).
// El cobro le crea la cuenta (CAP-73) porque un boleto vale al menos 1 XLM.
#[test]
fn el_cobro_crea_la_cuenta_de_quien_no_tenia() {
    let p = setup();
    p.env.ledger().set_base_reserve(5_000_000);
    let id = xdr::AccountId(xdr::PublicKey::PublicKeyTypeEd25519(xdr::Uint256([7; 32])));
    let rosa = Address::try_from_val(&p.env, &xdr::ScAddress::Account(id)).unwrap();

    p.contrato.cargar(&5);
    p.contrato.entregar(&1, &rosa, &3);
    assert_eq!(p.contrato.cobrar(&rosa), 3 * VALOR);
    assert_eq!(p.xlm.balance(&rosa), 3 * VALOR);
}

// Sin ninguna firma, la plata llega igual a Rosa y a nadie más.
#[test]
fn cualquiera_puede_pedir_el_cobro_pero_le_llega_al_recolector() {
    let p = setup();
    p.contrato.cargar(&5);
    p.contrato.entregar(&1, &p.rosa, &2);
    p.env.mock_auths(&[]); // desde acá no firma nadie
    assert_eq!(p.contrato.cobrar(&p.rosa), 2 * VALOR);
    assert_eq!(p.xlm.balance(&p.rosa), 2 * VALOR);
    assert_eq!(p.xlm.balance(&p.contrato.address), 3 * VALOR); // y no salió nada más
}

// 4. lo que no se puede

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn un_boleto_de_menos_de_1_xlm_falla() {
    let p = setup();
    let menos = 9_999_999i128;
    p.env.register(BoletoLleno, (&p.productor, &p.medidor, menos));
}

#[test]
fn un_boleto_de_1_xlm_justo_sirve() {
    let p = setup();
    let justo = 10_000_000i128;
    let id = p.env.register(BoletoLleno, (&p.productor, &p.medidor, justo));
    assert_eq!(BoletoLlenoClient::new(&p.env, &id).datos().valor, justo);
}

// Hay un tope de 10 000 XLM por boleto, así las cuentas nunca se pasan.
#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn un_boleto_de_mas_de_10_000_xlm_falla() {
    let p = setup();
    let mas = 100_000_000_001i128;
    p.env.register(BoletoLleno, (&p.productor, &p.medidor, mas));
}

#[test]
fn un_boleto_de_10_000_xlm_justo_sirve() {
    let p = setup();
    let justo = 100_000_000_000i128;
    let id = p.env.register(BoletoLleno, (&p.productor, &p.medidor, justo));
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
    assert_eq!(Error::MedidorInvalido as u32, 7);
}

#[test]
fn el_contrato_no_puede_ser_recolector() {
    let p = setup();
    p.contrato.cargar(&5);
    let r = p.contrato.try_entregar(&1, &p.contrato.address, &1);
    assert_eq!(r, Err(Ok(Error::RecolectorInvalido)));
}

// 5. la moneda

// Nadie elige la moneda al desplegar: el contrato usa la dirección de XLM de la red.
#[test]
fn la_moneda_siempre_es_xlm() {
    let p = setup();
    assert_eq!(p.contrato.datos().moneda, p.xlm.address);
    assert_eq!(p.xlm.symbol(), soroban_sdk::String::from_str(&p.env, "native"));
}

// 6. las firmas

#[test]
fn cada_uno_firma_lo_suyo() {
    let p = setup();
    p.contrato.cargar(&5);
    assert!(p.firmo(&p.productor, "cargar", (5u32,).into_val(&p.env)));
    p.contrato.entregar(&1, &p.rosa, &2);
    assert!(p.firmo(&p.medidor, "entregar", (1u32, &p.rosa, 2u32).into_val(&p.env)));
    p.contrato.retirar(&1);
    assert!(p.firmo(&p.productor, "retirar", (1u32,).into_val(&p.env)));
    let nuevo = Address::generate(&p.env);
    p.contrato.cambiar_medidor(&nuevo);
    assert!(p.firmo(&p.productor, "cambiar_medidor", (&nuevo,).into_val(&p.env)));
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
    let id = p.env.register(BoletoLleno, (&p.productor, &p.productor, VALOR));
    let contrato = BoletoLlenoClient::new(&p.env, &id);
    contrato.cargar(&5);
    contrato.entregar(&1, &p.rosa, &3);
    assert_eq!(contrato.try_retirar(&3), Err(Ok(Error::SinLibres)));
    assert_eq!(contrato.cobrar(&p.rosa), 3 * VALOR);
}

// 7. cambiar al medidor

// Si al medidor le roban la clave, el productor pone otro y el viejo ya no entrega.
#[test]
fn el_productor_cambia_al_medidor() {
    let p = setup();
    p.contrato.cargar(&5);
    p.contrato.entregar(&1, &p.rosa, &2);
    let nuevo = Address::generate(&p.env);
    p.contrato.cambiar_medidor(&nuevo);
    assert_eq!(p.contrato.datos().medidor, nuevo);

    p.solo_firma(&p.medidor, "entregar", (2u32, &p.luis, 1u32).into_val(&p.env));
    assert!(p.contrato.try_entregar(&2, &p.luis, &1).is_err()); // el viejo ya no puede

    p.solo_firma(&nuevo, "entregar", (2u32, &p.luis, 1u32).into_val(&p.env));
    p.contrato.entregar(&2, &p.luis, &1); // el nuevo sí
    assert_eq!(p.contrato.boletos(&p.rosa), 2); // lo de Rosa no cambió
    assert_eq!(p.contrato.boletos(&p.luis), 1);
}

// El contrato no puede ser medidor: nadie podría entregar.
#[test]
fn el_contrato_no_puede_ser_medidor() {
    let p = setup();
    let r = p.contrato.try_cambiar_medidor(&p.contrato.address);
    assert_eq!(r, Err(Ok(Error::MedidorInvalido)));
    assert_eq!(p.contrato.datos().medidor, p.medidor);
}

#[test]
fn el_medidor_no_se_puede_cambiar_solo() {
    let p = setup();
    let otro = Address::generate(&p.env);
    p.solo_firma(&p.medidor, "cambiar_medidor", (&otro,).into_val(&p.env));
    assert!(p.contrato.try_cambiar_medidor(&otro).is_err());
    assert_eq!(p.contrato.datos().medidor, p.medidor);
}

// 8. lo que queda a la vista

#[test]
fn se_ven_los_datos() {
    let p = setup();
    let esperado = Datos {
        productor: p.productor.clone(),
        medidor: p.medidor.clone(),
        moneda: p.xlm.address.clone(),
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

    let nuevo = Address::generate(&p.env);
    p.contrato.cambiar_medidor(&nuevo);
    let m = MedidorCambiado { anterior: p.medidor.clone(), medidor: nuevo };
    assert_eq!(avisos(), std::vec![m.to_xdr(&p.env, id)]);
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

// 9. miles de pasos al azar
//
// Se juega con el contrato al azar y, en paralelo, se lleva la cuenta en un modelo
// simple. Después de cada paso, el contrato y el modelo tienen que coincidir y la
// plata tiene que alcanzar justo para todos los boletos. Las semillas son fijas,
// así que si algo falla se puede repetir igual.

struct Azar(u64);

impl Azar {
    fn hasta(&mut self, n: u32) -> u32 {
        // xorshift64: alcanza para elegir pasos, no es para criptografía
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % u64::from(n)) as u32
    }
}

#[test]
fn al_azar_el_contrato_y_las_cuentas_siempre_coinciden() {
    for semilla in 1..=25u64 {
        let p = setup();
        let mut azar = Azar(semilla.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let recolectores = [p.rosa.clone(), p.luis.clone(), Address::generate(&p.env)];
        let (mut libres, mut ultima) = (0u32, 0u32);
        let mut suyos = [0u32; 3];
        let mut del_productor = PLATA_INICIAL;
        let mut cobrado = [0i128; 3];

        for _ in 0..80 {
            match azar.hasta(4) {
                0 => {
                    let n = azar.hasta(8); // a veces 0, a veces más de lo que alcanza
                    let r = p.contrato.try_cargar(&n);
                    if n == 0 {
                        assert_eq!(r, Err(Ok(Error::CantidadInvalida)));
                    } else if VALOR * i128::from(n) > del_productor {
                        assert!(r.is_err());
                    } else {
                        assert!(r.is_ok());
                        libres += n;
                        del_productor -= VALOR * i128::from(n);
                    }
                }
                1 => {
                    let i = azar.hasta(3) as usize;
                    let n = azar.hasta(6);
                    let numero = if azar.hasta(8) == 0 { ultima } else { ultima + 1 };
                    let r = p.contrato.try_entregar(&numero, &recolectores[i], &n);
                    if n == 0 {
                        assert_eq!(r, Err(Ok(Error::CantidadInvalida)));
                    } else if numero != ultima + 1 {
                        assert_eq!(r, Err(Ok(Error::NumeroInvalido)));
                    } else if n > libres {
                        assert_eq!(r, Err(Ok(Error::SinLibres)));
                    } else {
                        assert!(r.is_ok());
                        libres -= n;
                        suyos[i] += n;
                        ultima = numero;
                    }
                }
                2 => {
                    let i = azar.hasta(3) as usize;
                    let r = p.contrato.try_cobrar(&recolectores[i]);
                    if suyos[i] == 0 {
                        assert_eq!(r, Err(Ok(Error::NadaParaCobrar)));
                    } else {
                        assert_eq!(r, Ok(Ok(VALOR * i128::from(suyos[i]))));
                        cobrado[i] += VALOR * i128::from(suyos[i]);
                        suyos[i] = 0;
                    }
                }
                _ => {
                    let n = azar.hasta(5);
                    let r = p.contrato.try_retirar(&n);
                    if n == 0 {
                        assert_eq!(r, Err(Ok(Error::CantidadInvalida)));
                    } else if n > libres {
                        assert_eq!(r, Err(Ok(Error::SinLibres)));
                    } else {
                        assert!(r.is_ok());
                        libres -= n;
                        del_productor += VALOR * i128::from(n);
                    }
                }
            }

            // el contrato dice lo mismo que el modelo
            let entregados: u32 = suyos.iter().sum();
            assert_eq!(p.contrato.estado(), Estado { libres, entregados, ultima });
            for (i, quien) in recolectores.iter().enumerate() {
                assert_eq!(p.contrato.boletos(quien), suyos[i]);
                assert_eq!(p.xlm.balance(quien), cobrado[i]);
            }
            // la plata alcanza justo para todos los boletos, ni más ni menos
            let en_contrato = p.xlm.balance(&p.contrato.address);
            assert_eq!(en_contrato, VALOR * i128::from(libres + entregados));
            assert_eq!(p.xlm.balance(&p.productor), del_productor);
            // y no se crea ni se pierde plata
            assert_eq!(del_productor + en_contrato + cobrado.iter().sum::<i128>(), PLATA_INICIAL);
        }
    }
}
