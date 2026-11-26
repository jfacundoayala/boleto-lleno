pub const DIA: u32 = 17_280; // ledgers por día

// TTL: si quedan menos de 30 días, se estira a 120
pub const TTL_RENOVAR_SI_QUEDAN: u32 = 30 * DIA;
pub const TTL_ESTIRAR_HASTA: u32 = 120 * DIA;

pub const VALOR_MINIMO: i128 = 10_000_000; // 1 XLM, para que el cobro pueda crear la cuenta (CAP-73)
pub const VALOR_MAXIMO: i128 = 100_000_000_000; // 10 000 XLM

pub const XLM: [u8; 4] = [0, 0, 0, 0]; // Asset::Native en XDR
