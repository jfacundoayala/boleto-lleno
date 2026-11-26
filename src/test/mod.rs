//! Lo que comparten los tests: el contrato armado y las firmas.

extern crate std;

mod azar;
mod firmas;
mod flujo;
mod limites;
mod vista;

use crate::{BoletoLleno, BoletoLlenoClient};
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, MockAuth, MockAuthInvoke};
use soroban_sdk::token::TokenClient;
use soroban_sdk::{xdr, Address, Env, Symbol, TryFromVal, TryIntoVal, Val, Vec};
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

// el contrato de XLM de la red de prueba
fn crear_xlm(env: &Env) -> Address {
    let crear = xdr::HostFunction::CreateContract(xdr::CreateContractArgs {
        contract_id_preimage: xdr::ContractIdPreimage::Asset(xdr::Asset::Native),
        executable: xdr::ContractExecutable::StellarAsset,
    });
    env.host().invoke_function(crear).unwrap().try_into_val(env).unwrap()
}

// XLM no tiene mint: se crea la cuenta con saldo en el ledger de prueba
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

// XLM, el contrato y un productor con plata para 100 boletos
fn preparar() -> Prueba {
    let env = Env::default();
    env.mock_all_auths(); // todos firman

    let xlm = crear_xlm(&env);
    // el productor es una dirección generada para poder simular sus firmas
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
