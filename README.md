# minelts

Launcher ultraligero de Minecraft no premium, escrito en Rust con Tauri (HTML/CSS/JS vanilla).

## Características

- Modo offline (no premium): solo nombre de usuario
- Solo vanilla (releases, snapshots y versiones antiguas)
- RAM mínima y máxima configurables
- Argumentos JVM personalizados
- Instalar versión sin lanzar el juego
- Filtros de versiones en el sidebar (releases, snapshots, old)
- Descarga automática del JRE oficial de Mojang
- Descargas paralelas con verificación SHA1
- Panel de versiones instaladas
- Botones para abrir `.minecraft`, `/versions`, `logs` y `crash-reports`
- Atajos de teclado
- Ventana fija 900×600
- Binario optimizado (~3.6 MB release)

## Requisitos

- Windows 10/11 x64 con **WebView2** (preinstalado en la mayoría de equipos)
- [Rust](https://rustup.rs/) 1.75+

## Compilar

```powershell
cargo build --release
```

El ejecutable queda en `target\release\minelts.exe`.

No se requiere Node.js ni npm.

## Uso

1. Ejecuta `minelts.exe`
2. Introduce tu nombre de usuario (máx. 16 caracteres)
3. Ajusta RAM mín/máx y argumentos JVM si lo necesitas
4. Filtra versiones en el sidebar (releases, snapshots, old)
5. Selecciona una versión en la lista
6. Pulsa **Install** para descargar sin jugar, o **Launch** para instalar y abrir Minecraft

## Atajos de teclado

| Atajo | Acción |
|-------|--------|
| `Enter` | Launch (fuera de inputs) |
| `Ctrl+Enter` | Launch (siempre) |
| `Ctrl+Shift+I` | Install |
| `Ctrl+F` | Buscar versiones |
| `Ctrl+R` | Refrescar listas |
| `Ctrl+L` | Abrir carpeta logs |
| `Ctrl+Shift+C` | Abrir crash-reports |
| `?` | Mostrar atajos en la barra de estado |

## Estructura

```
minelts/
├── ui/              # Frontend estático (HTML/CSS/JS)
└── src-tauri/       # Backend Rust + Tauri
    └── src/         # Lógica del launcher
```

## Configuración

Se guarda en `%APPDATA%\minelts\config.json`:

- `username`, `version_id`
- `ram_min_mb`, `ram_max_mb` (512–16384 MB)
- `jvm_args` (flags extra separados por espacio)
- `filters`: `show_releases`, `show_snapshots`, `show_old`

Los datos del juego van a `%APPDATA%\.minecraft`.

Configuraciones antiguas con `ram_mb` / `show_snapshots` se migran automáticamente al cargar.

## Licencia

MIT
