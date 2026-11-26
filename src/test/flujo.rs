use super::*;
use crate::constants::VALOR_MINIMO;
use crate::{BoletoLleno, BoletoLlenoClient, Error, Estado};
use soroban_sdk::testutils::Ledger as _;
use soroban_sdk::{xdr, Address, TryFromVal};

#[test]
fn cargar_entregar_cobrar_y_retirar_lo_que_sobra() {
    let p = preparar();
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
    let p = preparar();
    p.contrato.cargar(&10);
    p.contrato.entregar(&1, &p.rosa, &2);
    p.contrato.entregar(&2, &p.luis, &1);
    p.contrato.entregar(&3, &p.rosa, &4);
    assert_eq!(p.contrato.cobrar(&p.rosa), 6 * VALOR);
    assert_eq!(p.contrato.boletos(&p.luis), 1); // lo de Luis no se toca
}

#[test]
fn sin_plata_no_hay_boleto() {
    let p = preparar();
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
    let p = preparar();
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
    let p = preparar();
    assert!(p.contrato.try_cargar(&101).is_err());
    assert_eq!(p.contrato.estado().libres, 0);
    assert_eq!(p.xlm.balance(&p.contrato.address), 0);
}

// Plata mandada por fuera no crea boletos: no se entregan sin cargarlos.
#[test]
fn la_plata_de_afuera_no_crea_boletos() {
    let p = preparar();
    p.xlm.transfer(&p.productor, &p.contrato.address, &(5 * VALOR));
    let r = p.contrato.try_entregar(&1, &p.rosa, &1);
    assert_eq!(r, Err(Ok(Error::SinLibres)));
}

// Una secuencia larga, revisando la regla después de cada paso.
#[test]
fn siempre_hay_plata_para_todos_los_boletos() {
    let p = preparar();
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

#[test]
fn no_se_cobra_dos_veces() {
    let p = preparar();
    p.contrato.cargar(&5);
    p.contrato.entregar(&1, &p.rosa, &2);
    p.contrato.cobrar(&p.rosa);
    assert_eq!(p.contrato.try_cobrar(&p.rosa), Err(Ok(Error::NadaParaCobrar)));
    assert_eq!(p.contrato.try_cobrar(&p.luis), Err(Ok(Error::NadaParaCobrar)));
    assert_eq!(p.xlm.balance(&p.rosa), 2 * VALOR);
}

// Rosa no tiene cuenta y la reserva es 0,5 XLM. Un boleto del valor mínimo
// (1 XLM) alcanza para que el cobro se la cree (CAP-73).
#[test]
fn el_cobro_crea_la_cuenta_de_quien_no_tenia() {
    let p = preparar();
    p.env.ledger().set_base_reserve(5_000_000);
    let id = xdr::AccountId(xdr::PublicKey::PublicKeyTypeEd25519(xdr::Uint256([7; 32])));
    let rosa = Address::try_from_val(&p.env, &xdr::ScAddress::Account(id)).unwrap();
    let id = p.env.register(BoletoLleno, (&p.productor, &p.medidor, VALOR_MINIMO));
    let contrato = BoletoLlenoClient::new(&p.env, &id);

    contrato.cargar(&1);
    contrato.entregar(&1, &rosa, &1);
    assert_eq!(contrato.cobrar(&rosa), VALOR_MINIMO);
    assert_eq!(p.xlm.balance(&rosa), VALOR_MINIMO);
}

// Sin ninguna firma, la plata llega igual a Rosa y a nadie más.
#[test]
fn cualquiera_puede_pedir_el_cobro_pero_le_llega_al_recolector() {
    let p = preparar();
    p.contrato.cargar(&5);
    p.contrato.entregar(&1, &p.rosa, &2);
    p.env.mock_auths(&[]); // desde acá no firma nadie
    assert_eq!(p.contrato.cobrar(&p.rosa), 2 * VALOR);
    assert_eq!(p.xlm.balance(&p.rosa), 2 * VALOR);
    assert_eq!(p.xlm.balance(&p.contrato.address), 3 * VALOR); // y no salió nada más
}
