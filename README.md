# minelts

Launcher ultraligero de Minecraft no premium, escrito en Rust con Tauri (HTML/CSS/JS vanilla).

## Características

- Modo offline (no premium): solo nombre de usuario
- Solo vanilla (release + snapshots)
- Descarga automática del JRE oficial de Mojang
- Descargas paralelas con verificación SHA1
- Panel de versiones instaladas
- Botones para abrir `.minecraft` y `/versions`
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
3. Selecciona una versión (clic en instaladas o en el desplegable)
4. Pulsa **JUGAR**

## Estructura

```
minelts/
├── ui/              # Frontend estático (HTML/CSS/JS)
└── src-tauri/       # Backend Rust + Tauri
    └── src/         # Lógica del launcher
```

## Configuración

Se guarda en `%APPDATA%\minelts\config.json`.

Los datos del juego van a `%APPDATA%\.minecraft`.

## Licencia

MIT
