//! Números fijos del contrato.

/// Ledgers en un día (uno cada ~5 segundos).
pub const DIA: u32 = 17_280;

/// Si a un dato le quedan menos de 30 días de vida (TTL), se renueva.
pub const TTL_RENOVAR_SI_QUEDAN: u32 = 30 * DIA;

/// Al renovar, la vida se estira a unos 120 días.
pub const TTL_ESTIRAR_HASTA: u32 = 120 * DIA;

/// 1 XLM: mínimo para activar una cuenta nueva al cobrar, según CAP-73.
pub const VALOR_MINIMO: i128 = 10_000_000;

/// 10 000 XLM: tope por boleto, así las cuentas nunca se pasan.
pub const VALOR_MAXIMO: i128 = 100_000_000_000;

/// XLM (`Asset::Native`) escrito en el formato de la red.
pub const XLM: [u8; 4] = [0, 0, 0, 0];
