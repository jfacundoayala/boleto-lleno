(() => {
  const S = window.StellarSdk;
  if (!S) {
    document.getElementById("vivo").classList.add("error");
    document.getElementById("vivo-texto").textContent = "Sin conexión";
    document.getElementById("actualizado").textContent = "No se pudo cargar la herramienta de Stellar. Vuelva a cargar la página en un ratito.";
    document.getElementById("historia").innerHTML = '<li class="cargando">No se pudo leer la red.</li>';
    return;
  }
  const CONTRATO = "CBWS4OR7LPKIK6TTYCTGWP2X2Y24VKSLAC6ALWIAUREGMYPKMKXPLAYC";
  const RPC = "https://soroban-testnet.stellar.org";
  const RED = S.Networks.TESTNET;
  const XLM_SAC = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
  const RECOLECTORES = [
    { nombre: "Rosa", cuenta: "GCG32KQZE7FK4TDWGQGXYO3QMOBFPWEJMYM7XH7JYNAKZJ36VF223VSH", nota: "recolectora" },
    { nombre: "Ana", cuenta: "GBSOVBN5TFUASEK6TLDTKPME7O5OSITLUD47RG4UBR7W74WGOEGTVHYO", nota: "el cobro le creó la cuenta" },
  ];
  const EXPERT = "https://stellar.expert/explorer/testnet";
  const STROOPS = 10_000_000n;
  // sha256 del wasm de la versión 3.0.0, el que compiló y firmó GitHub
  const WASM = "da3d6878439ac63733016239a410f75fa4d9ab77d8df4282dd975cd2f6e8fc98";

  // La prueba del README. La red guarda los eventos solo unos 7 días,
  // así que estos quedan siempre a la vista. Los nuevos se leen en vivo.
  const PRUEBA = [
    { tipo: "publicar", hash: "b44c1ecac254f33af95610ae634969bd40c2eab50340c89978f27db18c6e4f91", fecha: "2026-11-26T03:59:18Z", texto: "Se publica el contrato", detalle: "Boletos de 2 XLM" },
    { tipo: "cargar", hash: "02980e603ba46c6e3e43fe398bbffe16912f88ae928be15689c40c72e7585d1c", fecha: "2026-11-26T03:59:23Z", texto: "El productor carga 5 boletos", detalle: "Entran 10 XLM" },
    { tipo: "entregar", hash: "5fc949b94da576506f2360c06742398bbc3b83f7e35b44c069a99c6e4afd9f30", fecha: "2026-11-26T03:59:28Z", texto: "El medidor entrega 3 boletos a Rosa", detalle: "Entrega número 1" },
    { tipo: "retirar", hash: "f0b96ea8c1cd39b9ac4b92c6e4fd85b91f0ff5e739a1074879c888ecca61ef87", fecha: "2026-11-26T03:59:33Z", texto: "El productor retira 2 boletos", detalle: "Vuelven 4 XLM" },
    { tipo: "cobrar", hash: "afe4666a075b7ea546c2c29ebfddec07dd982e95d28be865f4a10591f388edf3", fecha: "2026-11-26T03:59:38Z", texto: "Se cobran 3 boletos para Rosa", detalle: "Le llegan 6 XLM" },
    { tipo: "cargar", hash: "b474625641e8791549d4bee98c9dcf20caa2d6a464d462847e09dcd00af07836", fecha: "2026-11-26T03:59:43Z", texto: "El productor carga 1 boleto", detalle: "Entran 2 XLM" },
    { tipo: "entregar", hash: "5b355aa194896b4b9a2f7a014921ec1e84c98a170076edc83ebc229a7446be08", fecha: "2026-11-26T03:59:48Z", texto: "El medidor entrega 1 boleto a Ana", detalle: "Entrega número 2" },
    { tipo: "cobrar", hash: "8ac2a38f233e633afbe88babea625b697b99fae560024ad6e92be66a51b25047", fecha: "2026-11-26T03:59:53Z", texto: "Se cobra 1 boleto para Ana", detalle: "Le llegan 2 XLM y se crea su cuenta" },
    { tipo: "cargar", hash: "8e3213482d86276f38cb976e6e76584e04256811f7e4cd20b618599756b6f599", fecha: "2026-11-26T03:59:58Z", texto: "El productor carga 4 boletos", detalle: "Entran 8 XLM" },
    { tipo: "medidor", hash: "e1f54140bf09746b3d8b0e9272a307f4f94252cc5945d132e9363a943ccbe4d4", fecha: "2026-11-26T04:00:03Z", texto: "El productor cambia el medidor a GC4JW6…DXBD", detalle: "Antes era GCB5BD…IAA2" },
    { tipo: "entregar", hash: "e5ada3e125e7bc361c937d1483064c2a8b1ada7ed714721ff23fce3a884a3b45", fecha: "2026-11-26T04:00:08Z", texto: "El medidor entrega 2 boletos a Rosa", detalle: "Entrega número 3" },
  ];

  const $ = (id) => document.getElementById(id);
  const corto = (a) => a.slice(0, 6) + "…" + a.slice(-4);
  const NOMBRES = Object.fromEntries(RECOLECTORES.map((r) => [r.cuenta, r.nombre]));
  const nombre = (a) => NOMBRES[a] || corto(a);
  const xlm = (stroops) => {
    const v = BigInt(stroops);
    const ent = v / STROOPS, dec = v % STROOPS;
    return dec === 0n ? ent.toString() : (Number(v) / 1e7).toLocaleString("es-CR", { maximumFractionDigits: 7 });
  };
  const fecha = (iso) => new Date(iso).toLocaleString("es-CR", { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });

  const ICONOS = {
    publicar: '<path d="M12 3v12"/><path d="m7 8 5-5 5 5"/><path d="M5 21h14"/>',
    cargar: '<path d="M12 5v14"/><path d="M5 12h14"/>',
    entregar: '<path d="M5 12h14"/><path d="m13 5 7 7-7 7"/>',
    cobrar: '<path d="M20 6 9 17l-5-5"/>',
    retirar: '<path d="M19 12H5"/><path d="m11 19-7-7 7-7"/>',
    medidor: '<path d="M17 3l4 4-4 4"/><path d="M21 7H9"/><path d="M7 21l-4-4 4-4"/><path d="M3 17h12"/>',
  };
  const icono = (t) => `<svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">${ICONOS[t]}</svg>`;
  const boleto = (color) => `<svg viewBox="0 0 44 28" aria-hidden="true"><path d="M4 0h36a4 4 0 0 1 4 4v6a4 4 0 0 0 0 8v6a4 4 0 0 1-4 4H4a4 4 0 0 1-4-4v-6a4 4 0 0 0 0-8V4a4 4 0 0 1 4-4Z" fill="${color}"/><path d="M15 14.5l4 4 9-9" fill="none" stroke="#F4E9DA" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/></svg>`;

  let cliente, monedaCliente, servidor;

  async function conectar() {
    servidor = new S.rpc.Server(RPC);
    cliente = await S.contract.Client.from({ contractId: CONTRATO, rpcUrl: RPC, networkPassphrase: RED });
  }

  function vivo(ok, texto) {
    $("vivo").classList.toggle("error", !ok);
    $("vivo-texto").innerHTML = texto;
  }

  async function leerEstado() {
    const [e, d] = await Promise.all([cliente.estado(), cliente.datos()]);
    const estado = e.result, datos = d.result;
    if (!monedaCliente) {
      monedaCliente = await S.contract.Client.from({ contractId: datos.moneda, rpcUrl: RPC, networkPassphrase: RED });
    }
    const saldo = (await monedaCliente.balance({ id: CONTRATO })).result;
    return { estado, datos, saldo: BigInt(saldo) };
  }

  function pintarEstado({ estado, datos, saldo }) {
    const valor = BigInt(datos.valor);
    $("n-libres").textContent = estado.libres;
    $("n-entregados").textContent = estado.entregados;
    $("n-valor").textContent = xlm(valor);
    $("n-saldo").textContent = xlm(saldo);

    const caja = $("boletos");
    const total = estado.libres + estado.entregados;
    if (total === 0) {
      caja.innerHTML = '<span class="vacio">En este momento no hay boletos: todo se cobró o se retiró.</span>';
    } else {
      const max = 40;
      const html = [];
      for (let i = 0; i < Math.min(estado.libres, max); i++) html.push(boleto("var(--green)"));
      for (let i = 0; i < Math.min(estado.entregados, max); i++) html.push(boleto("var(--coffee)"));
      if (estado.libres > max || estado.entregados > max) html.push(`<span class="vacio">y más</span>`);
      caja.innerHTML = html.join("");
    }

    const necesita = valor * BigInt(total);
    const ok = saldo >= necesita;
    $("regla").classList.toggle("mal", !ok);
    $("regla-texto").textContent = ok
      ? "Todos los boletos tienen su plata guardada."
      : "La plata del contrato no alcanza para todos los boletos.";
    marcar("p-plata", ok, `El contrato tiene ${xlm(saldo)} XLM y los boletos necesitan ${xlm(necesita)} XLM.`);
    $("regla-cuenta").textContent = `Hay ${xlm(saldo)} XLM y hacen falta ${xlm(necesita)} XLM (${total} boleto${total === 1 ? "" : "s"} × ${xlm(valor)} XLM).`;

    $("d-productor").innerHTML = `<a href="${EXPERT}/account/${datos.productor}">${corto(datos.productor)}</a>`;
    $("d-medidor").innerHTML = `<a href="${EXPERT}/account/${datos.medidor}">${corto(datos.medidor)}</a>`;
    $("d-moneda").innerHTML = datos.moneda === XLM_SAC
      ? `XLM, la moneda de Stellar (<a class="mono" href="${EXPERT}/contract/${datos.moneda}">${corto(datos.moneda)}</a>)`
      : `<a class="mono" href="${EXPERT}/contract/${datos.moneda}">${corto(datos.moneda)}</a>`;

    $("actualizado").textContent = `Leído de la red: ${new Date().toLocaleTimeString("es-CR", { hour: "2-digit", minute: "2-digit", second: "2-digit" })}`;
  }

  // Los eventos se leen de a páginas. La primera vez desde lo más viejo que
  // guarda la red (unos 7 días). Después solo lo nuevo, desde el último cursor.
  const FILTRO = [{ type: "contract", contractIds: [CONTRATO] }];
  const vistos = new Map(PRUEBA.map((p) => [p.hash, p]));
  let cursor = null;
  let valorBoleto = null;

  function traducir(ev) {
    const tema = ev.topic.map((t) => S.scValToNative(t));
    const v = S.scValToNative(ev.value);
    const quien = tema[1] ? nombre(tema[1]) : "";
    if (tema[0] === "medidor_cambiado") {
      return { hash: ev.txHash, fecha: ev.ledgerClosedAt, tipo: "medidor", texto: `El productor cambia el medidor a ${corto(v.medidor)}`, detalle: `Antes era ${corto(v.anterior)}` };
    }
    const n = v.boletos;
    const s = n === 1 ? "" : "s";
    const plata = valorBoleto ? xlm(valorBoleto * BigInt(n)) + " XLM" : "";
    const base = { hash: ev.txHash, fecha: ev.ledgerClosedAt };
    switch (tema[0]) {
      case "cargado": return { ...base, tipo: "cargar", texto: `El productor carga ${n} boleto${s}`, detalle: plata && `Entran ${plata}` };
      case "entregado": return { ...base, tipo: "entregar", texto: `El medidor entrega ${n} boleto${s} a ${quien}`, detalle: `Entrega número ${v.numero}` };
      case "cobrado": return { ...base, tipo: "cobrar", texto: `Se cobran ${n} boleto${s} para ${quien}`, detalle: `Le llegan ${xlm(v.monto)} XLM` };
      case "retirado": return { ...base, tipo: "retirar", texto: `El productor retira ${n} boleto${s}`, detalle: plata && `Vuelven ${plata}` };
      default: return null;
    }
  }

  async function leerEventos() {
    let pedido;
    if (cursor) {
      pedido = { cursor, filters: FILTRO, limit: 100 };
    } else {
      const salud = await servidor.getHealth();
      pedido = { startLedger: salud.oldestLedger + 10, filters: FILTRO, limit: 100 };
    }
    const nuevos = [];
    for (let pagina = 0; pagina < 40; pagina++) {
      const r = await servidor.getEvents(pedido);
      for (const ev of r.events) {
        const e = traducir(ev);
        if (e && !vistos.has(e.hash)) { vistos.set(e.hash, e); nuevos.push(e.hash); }
      }
      if (!r.cursor) break;
      cursor = r.cursor;
      const hasta = Number(BigInt(r.cursor.split("-")[0]) >> 32n);
      if (hasta >= r.latestLedger - 1 && r.events.length < 100) break;
      pedido = { cursor, filters: FILTRO, limit: 100 };
    }
    return new Set(nuevos);
  }

  // Solo se animan los movimientos nuevos, así la lista no parpadea.
  function pintarHistoria(nuevos = new Set()) {
    const lista = [...vistos.values()].sort((a, b) => new Date(b.fecha) - new Date(a.fecha));
    $("historia").innerHTML = lista.map((e) => `
      <li${nuevos.has(e.hash) ? ' class="aparece"' : ""}>
        <span class="marca m-${e.tipo}">${icono(e.tipo)}</span>
        <div><div class="t">${e.texto}</div><div class="q">${fecha(e.fecha)}${e.detalle ? " · " + e.detalle : ""}</div></div>
        <a class="tx" href="${EXPERT}/tx/${e.hash}" aria-label="Ver la transacción ${e.hash.slice(0, 8)}">${e.hash.slice(0, 8)} ↗</a>
      </li>`).join("");
  }

  function marcar(id, ok, texto) {
    $(id).classList.toggle("ok", ok === true);
    $(id).classList.toggle("mal", ok === false);
    $(id + "-texto").textContent = texto;
  }

  // Lee qué código tiene el contrato en la red y lo compara con el de la versión.
  let codigoListo = false;
  async function comprobarCodigo() {
    if (codigoListo) return;
    try {
      const clave = new S.Contract(CONTRATO).getFootprint();
      const r = await servidor.getLedgerEntries(clave);
      const hash = String(r.entries[0].val.contractData.val.instance.executable.wasmHash);
      codigoListo = true;
      marcar("p-codigo", hash === WASM, hash === WASM
        ? `El código en la red es el mismo que compiló GitHub (${hash.slice(0, 8)}…${hash.slice(-4)}).`
        : `No coincide: en la red hay ${hash.slice(0, 8)}… y la versión es ${WASM.slice(0, 8)}…`);
    } catch (err) {
      console.error(err);
      marcar("p-codigo", null, "No se pudo comparar ahorita. Se puede revisar en los enlaces.");
    }
  }

  async function pintarConocidos() {
    const filas = await Promise.all(RECOLECTORES.map(async (r) => {
      const n = (await cliente.boletos({ recolector: r.cuenta })).result;
      const cuenta = n === 0 ? "nada sin cobrar" : `${n} boleto${n === 1 ? "" : "s"} sin cobrar`;
      return `<li><button type="button" data-cuenta="${r.cuenta}"><span><b>${r.nombre}</b> · ${r.nota}</span><small>${cuenta}</small></button></li>`;
    }));
    $("conocidos").innerHTML = filas.join("");
  }

  let enCurso = false;
  async function actualizar() {
    if (enCurso) return;
    enCurso = true;
    const boton = $("refrescar");
    boton.disabled = true;
    try {
      if (!cliente) await conectar();
      const lectura = await leerEstado();
      valorBoleto = BigInt(lectura.datos.valor);
      pintarEstado(lectura);
      await pintarConocidos();
      comprobarCodigo();
      vivo(true, 'En vivo<span class="texto-largo"> · Testnet</span>');
    } catch (err) {
      console.error(err);
      vivo(false, "Sin conexión");
      $("actualizado").textContent = "No se pudo leer la red. Intente de nuevo en un ratito.";
      if (!valorBoleto) {
        $("regla-texto").textContent = "Todavía no se pudo leer el contrato.";
        marcar("p-plata", null, "Todavía no se pudo leer el saldo.");
      }
    }
    try {
      pintarHistoria(servidor ? await leerEventos() : undefined);
    } catch (err) {
      console.error(err);
      cursor = null; // si el cursor venció, la próxima vez se lee de nuevo
      pintarHistoria();
    } finally {
      boton.disabled = false;
      enCurso = false;
    }
  }

  async function consultar(cuenta) {
    const caja = $("respuesta");
    caja.classList.add("ver");
    cuenta = cuenta.trim();
    if (!S.StrKey.isValidEd25519PublicKey(cuenta) && !S.StrKey.isValidContract(cuenta)) {
      caja.textContent = "Esa no parece una cuenta de Stellar. Empieza con G (o con C si es un contrato) y tiene 56 letras y números.";
      return;
    }
    caja.innerHTML = '<span class="cargando">Consultando…</span>';
    try {
      if (!cliente) await conectar();
      const n = (await cliente.boletos({ recolector: cuenta })).result;
      caja.innerHTML = n === 0
        ? `<b>0</b> boletos sin cobrar.<div class="chico">Si tenía boletos, ya los cobró.</div>`
        : `<b>${n}</b> boleto${n === 1 ? "" : "s"} sin cobrar, con su plata apartada.`;
    } catch (err) {
      console.error(err);
      caja.innerHTML = "No se pudo consultar la red. Intente de nuevo en un ratito.";
    }
  }

  $("refrescar").addEventListener("click", actualizar);
  $("consulta").addEventListener("submit", (e) => { e.preventDefault(); consultar($("cuenta").value); });
  $("conocidos").addEventListener("click", (e) => {
    const b = e.target.closest("button[data-cuenta]");
    if (b) { $("cuenta").value = b.dataset.cuenta; consultar(b.dataset.cuenta); }
  });

  pintarHistoria(); // la prueba fija se ve enseguida; lo nuevo llega después
  actualizar();
  setInterval(() => { if (!document.hidden) actualizar(); }, 60_000);
})();
