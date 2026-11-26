//! Lo que el contrato rechaza.

use super::*;
use crate::constants::{VALOR_MAXIMO, VALOR_MINIMO};
use crate::{BoletoLleno, BoletoLlenoClient, Error};

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn un_boleto_de_menos_de_1_xlm_falla() {
    let p = preparar();
    p.env.register(BoletoLleno, (&p.productor, &p.medidor, VALOR_MINIMO - 1));
}

#[test]
fn un_boleto_de_1_xlm_justo_sirve() {
    let p = preparar();
    let id = p.env.register(BoletoLleno, (&p.productor, &p.medidor, VALOR_MINIMO));
    assert_eq!(BoletoLlenoClient::new(&p.env, &id).datos().valor, VALOR_MINIMO);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn un_boleto_de_mas_de_10_000_xlm_falla() {
    let p = preparar();
    p.env.register(BoletoLleno, (&p.productor, &p.medidor, VALOR_MAXIMO + 1));
}

#[test]
fn un_boleto_de_10_000_xlm_justo_sirve() {
    let p = preparar();
    let id = p.env.register(BoletoLleno, (&p.productor, &p.medidor, VALOR_MAXIMO));
    assert_eq!(BoletoLlenoClient::new(&p.env, &id).datos().valor, VALOR_MAXIMO);
}

#[test]
fn cero_boletos_falla() {
    let p = preparar();
    p.contrato.cargar(&5);
    assert_eq!(p.contrato.try_cargar(&0), Err(Ok(Error::CantidadInvalida)));
    assert_eq!(p.contrato.try_retirar(&0), Err(Ok(Error::CantidadInvalida)));
    let r = p.contrato.try_entregar(&1, &p.rosa, &0);
    assert_eq!(r, Err(Ok(Error::CantidadInvalida)));
}

// Se cortó la señal y el medidor mandó dos veces la misma entrega: vale una.
#[test]
fn la_misma_entrega_no_cuenta_dos_veces() {
    let p = preparar();
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
    let p = preparar();
    p.contrato.cargar(&5);
    let r = p.contrato.try_entregar(&1, &p.contrato.address, &1);
    assert_eq!(r, Err(Ok(Error::RecolectorInvalido)));
}
