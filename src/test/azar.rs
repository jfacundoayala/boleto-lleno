// Pasos al azar contra un modelo. Las semillas son fijas, así se puede repetir.

use super::*;
use crate::{Error, Estado};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::Address;

struct Azar(u64);

impl Azar {
    fn hasta(&mut self, n: u32) -> u32 {
        // xorshift64
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % u64::from(n)) as u32
    }
}

#[test]
fn al_azar_el_contrato_y_las_cuentas_siempre_coinciden() {
    for semilla in 1..=25u64 {
        let p = preparar();
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

            let entregados: u32 = suyos.iter().sum();
            assert_eq!(p.contrato.estado(), Estado { libres, entregados, ultima });
            for (i, quien) in recolectores.iter().enumerate() {
                assert_eq!(p.contrato.boletos(quien), suyos[i]);
                assert_eq!(p.xlm.balance(quien), cobrado[i]);
            }
            let en_contrato = p.xlm.balance(&p.contrato.address);
            assert_eq!(en_contrato, VALOR * i128::from(libres + entregados));
            assert_eq!(p.xlm.balance(&p.productor), del_productor);
            // no se crea ni se pierde plata
            assert_eq!(del_productor + en_contrato + cobrado.iter().sum::<i128>(), PLATA_INICIAL);
        }
    }
}
