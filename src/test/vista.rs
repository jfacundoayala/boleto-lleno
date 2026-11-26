extern crate std;

use super::*;
use crate::constants::{DIA, TTL_ESTIRAR_HASTA};
use crate::storage::DataKey;
use crate::{Cargado, Cobrado, Datos, Entregado, MedidorCambiado, Retirado};
use soroban_sdk::testutils::storage::{Instance as _, Persistent as _};
use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _};
use soroban_sdk::{Address, Event as _};

// La moneda es siempre XLM, sin que nadie la elija.
#[test]
fn se_ven_los_datos_y_la_moneda_es_xlm() {
    let p = preparar();
    assert_eq!(p.xlm.symbol(), soroban_sdk::String::from_str(&p.env, "native"));
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
    let p = preparar();
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

// La primera entrega deja 120 días de vida. Cuando quedan menos de 30, otra entrega
// la vuelve a estirar.
#[test]
fn renueva_el_ttl() {
    let p = preparar();
    p.contrato.cargar(&5);
    let vida = || {
        p.env.as_contract(&p.contrato.address, || {
            let s = p.env.storage();
            let clave = DataKey::Boletos(p.rosa.clone());
            (s.instance().get_ttl(), s.persistent().get_ttl(&clave))
        })
    };
    p.contrato.entregar(&1, &p.rosa, &2);
    assert_eq!(vida(), (TTL_ESTIRAR_HASTA, TTL_ESTIRAR_HASTA));

    p.env.ledger().with_mut(|l| l.sequence_number += 91 * DIA); // pasan 91 días
    p.contrato.entregar(&2, &p.rosa, &1);
    assert_eq!(vida(), (TTL_ESTIRAR_HASTA, TTL_ESTIRAR_HASTA));
}
