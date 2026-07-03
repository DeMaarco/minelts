# minelts

Launcher ultraligero y **portable** de Minecraft no premium (Rust + Tauri, UI vanilla).

## Características

- **Portable**: un solo `minelts.exe`; `config.json` y `.minecraft/` se crean junto al ejecutable
- Modo offline (no premium): solo nombre de usuario
- Solo vanilla (releases, snapshots y versiones antiguas)
- RAM min/máx con sliders (tope = RAM del sistema)
- Argumentos JVM personalizados
- Instalar versión sin lanzar
- Filtros en sidebar, atajos de teclado, ventana fija 900×600
- Descarga automática del JRE de Mojang, paralela con SHA1

## Requisitos

- Windows 10/11 x64 con **WebView2**
- Para compilar: [Rust](https://rustup.rs/) 1.75+

## Distribución (única opción)

Solo necesitas **`minelts.exe`**. Colócalo donde quieras (USB, carpeta, escritorio).

En el primer uso se crean automáticamente:

```
minelts.exe
config.json      ← ajustes del launcher
.minecraft/      ← juego, versiones, assets
```

## Compilar

```powershell
.\release.ps1
```

Genera `dist\minelts.exe` listo para distribuir.

O manualmente:

```powershell
cargo build --release
```

Salida: `target\release\minelts.exe`

## Uso

1. Ejecuta `minelts.exe`
2. Usuario (máx. 16 caracteres), RAM y JVM si quieres
3. Elige versión en el sidebar
4. **Install** (descargar) o **Launch** (jugar)

## Atajos

| Atajo | Acción |
|-------|--------|
| `Enter` | Launch |
| `Ctrl+Enter` | Launch (siempre) |
| `Ctrl+Shift+I` | Install |
| `Ctrl+F` | Buscar versiones |
| `Ctrl+R` | Refrescar |
| `Ctrl+L` | Logs |
| `Ctrl+Shift+C` | Crash-reports |
| `?` | Ayuda de atajos |

## Licencia

MIT
