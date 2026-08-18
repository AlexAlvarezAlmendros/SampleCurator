# spike/ — código desechable de la Fase 0

Esto **no es la app**. Es el instrumento de medida que respondió a la pregunta del gate:
*¿se sostiene la premisa de SampleCurator en esta máquina?*

Veredicto: **GO**. Los números están en [`../docs/adr/0005-resultados-del-spike.md`](../docs/adr/0005-resultados-del-spike.md).

Se conserva porque los benchmarks siguen siendo útiles como referencia mientras se construye la
app, y porque el generador de biblioteca sintética hace falta en las Fases 2, 3 y 6. Cuando
`src-tauri/` tenga sus propios benches de `criterion`, esta carpeta se borra.

## Uso

```bash
cd spike

# 1. Biblioteca sintética (50.000 archivos ≈ 4,2 GB, con 2.000 en flac/mp3/ogg/aiff)
cargo run --release -- gen --out ~/.cache/samplecurator-spike/lib50k --count 50000 --convert-pct 4

# 2. Todo de una vez, con veredicto final
cargo run --release -- all --lib ~/.cache/samplecurator-spike/lib50k

# 3. Por partes
cargo run --release -- bench-latency   --lib DIR --shots 200 --buffer 256   # añade --cold para incluir la decodificación
cargo run --release -- bench-decode    --lib DIR --files 600
cargo run --release -- bench-resample  --lib DIR --files 200
cargo run --release -- bench-retrigger                                       # no necesita biblioteca
cargo run --release -- bench-scan      --lib DIR
cargo run --release -- bench-peaks     --lib DIR --files 3000

# 4. La prueba que de verdad importa: el oído
cargo run --release -- play --lib DIR    # ↓/↑ navegar (suena solo) · espacio repetir · q salir
```

Para un escaneo honesto en frío, tira las cachés antes:
`sync && sudo sh -c 'echo 3 > /proc/sys/vm/drop_caches'`

## Diagnóstico de audio

```bash
cargo run --release -- probe --secs 8      # mantiene un tono para interrogar a PipeWire desde fuera
cargo run --release -- mic-check --secs 4  # qué está oyendo realmente la entrada
cargo run --release -- bench-loopback      # loopback acústico (no funciona en este equipo: ver ADR-0005 §1)
```

## Qué de aquí sobrevive a la app

- `engine.rs` — la separación hilo de control / callback, el `rtrb` de mandos y **el ring de
  basura para los `Arc`** (soltar el último Arc en el callback libera memoria en tiempo real).
- `decode.rs` — el bucle de symphonia y el buffer f32 intercalado.
- `scan.rs` — el recorrido con jwalk, los lotes de 1.000 en transacción y el formato de picos
  (2 bytes por bucket).
- `gen.rs` — el generador, que hará falta para los benches de las Fases 2, 3 y 6.

Lo que **no** sobrevive: el parseo de argumentos a mano, el remuestreo lineal (la app usa rubato)
y la ausencia total de manejo de errores hacia el usuario.
