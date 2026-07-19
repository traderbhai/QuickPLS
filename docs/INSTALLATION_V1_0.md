# QuickPLS 1.0.0 Installation

## Windows Executable

The portable executable is:

```text
target/release/quickpls-desktop.exe
```

It launches offline and does not require a dev server.

## Windows Installer

The NSIS installer is:

```text
target/release/bundle/nsis/QuickPLS_1.0.0_x64-setup.exe
```

The installer is unsigned unless a code-signing certificate is supplied and audited. Windows SmartScreen may warn before installation.

## Runtime Requirements

- Windows desktop environment.
- No R installation required.
- No Python installation required.
- No account, activation server, telemetry, cloud sync, or remote computation.

R, Rscript, lavaan, cSEM, seminr, plspm, NumPy, and related tools are development validation tools only.
