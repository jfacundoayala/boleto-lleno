# Cambios

## 3.0.0

Contrato en Testnet: `CBWS4OR7LPKIK6TTYCTGWP2X2Y24VKSLAC6ALWIAUREGMYPKMKXPLAYC`

- `entregar` y `retirar` revisan primero la cantidad, igual que `cargar`.
- Los módulos son privados. Hacia afuera solo sale lo que exporta `lib.rs`.
- La versión queda atada a su código, con compilado verificado.

## 2.0.0

Contrato en Testnet: `CBZ47VWCFR2MP6SOQEJVVP3L2XXDIK3IMST5MEQFMBFZOCD7QVXCEYIQ`

- La moneda es siempre XLM. Ya no se elige al publicar.
- El productor puede cambiar el medidor.
- El contrato se separa en módulos.
- Pruebas con pasos al azar.

## 1.0.0

Contrato en Testnet: `CCKXXTVEZXKXANOEJBYKLSBWOLR5CZ72I4KBJY27QIN3DKAK6BSXIHOI`

- Primera versión: cargar, entregar, cobrar y retirar.
