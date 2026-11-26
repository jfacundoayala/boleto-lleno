use super::*;
use crate::{BoletoLleno, BoletoLlenoClient, Error};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, IntoVal};

#[test]
fn cada_uno_firma_lo_suyo() {
    let p = preparar();
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
    let p = preparar();
    p.contrato.cargar(&5);
    p.solo_firma(&p.productor, "entregar", (1u32, &p.rosa, 2u32).into_val(&p.env));
    assert!(p.contrato.try_entregar(&1, &p.rosa, &2).is_err());
    assert_eq!(p.contrato.boletos(&p.rosa), 0);
}

#[test]
fn el_medidor_no_puede_cargar() {
    let p = preparar();
    p.solo_firma(&p.medidor, "cargar", (5u32,).into_val(&p.env));
    assert!(p.contrato.try_cargar(&5).is_err());
    assert_eq!(p.contrato.estado().libres, 0);
}

#[test]
fn el_medidor_no_puede_retirar() {
    let p = preparar();
    p.contrato.cargar(&5);
    p.solo_firma(&p.medidor, "retirar", (2u32,).into_val(&p.env));
    assert!(p.contrato.try_retirar(&2).is_err());
    assert_eq!(p.contrato.estado().libres, 5);
}

// En una finca pequeña el productor y el medidor pueden ser la misma cuenta.
#[test]
fn con_una_sola_cuenta_lo_entregado_sigue_protegido() {
    let p = preparar();
    let id = p.env.register(BoletoLleno, (&p.productor, &p.productor, VALOR));
    let contrato = BoletoLlenoClient::new(&p.env, &id);
    contrato.cargar(&5);
    contrato.entregar(&1, &p.rosa, &3);
    assert_eq!(contrato.try_retirar(&3), Err(Ok(Error::SinLibres)));
    assert_eq!(contrato.cobrar(&p.rosa), 3 * VALOR);
}

// le roban la clave al medidor: el productor pone otro
#[test]
fn el_productor_cambia_al_medidor() {
    let p = preparar();
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

#[test]
fn el_contrato_no_puede_ser_medidor() {
    let p = preparar();
    let r = p.contrato.try_cambiar_medidor(&p.contrato.address);
    assert_eq!(r, Err(Ok(Error::MedidorInvalido)));
    assert_eq!(p.contrato.datos().medidor, p.medidor);
}

#[test]
fn el_medidor_no_se_puede_cambiar_solo() {
    let p = preparar();
    let otro = Address::generate(&p.env);
    p.solo_firma(&p.medidor, "cambiar_medidor", (&otro,).into_val(&p.env));
    assert!(p.contrato.try_cambiar_medidor(&otro).is_err());
    assert_eq!(p.contrato.datos().medidor, p.medidor);
}
