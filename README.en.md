| [中文](README.md) | English |
| ----------------- | ------- |

<div align="center">

# DSH Desktop

A lightweight, cross-platform desktop application for [DeepSeek Harness (DSH)](https://github.com/deepseek-ai/deepseek-harness).

Launch DSH directly with the built-in runtime, connect to an existing service, or start DSH via system PATH, custom executable, or npx.

[![Latest Release](https://img.shields.io/github/v/release/Leawind/dsh-desktop?display_name=tag&sort=semver)](https://github.com/Leawind/dsh-desktop/releases)
![Supported Platforms](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-4c8bf5)
[![CI](https://github.com/Leawind/dsh-desktop/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Leawind/dsh-desktop/actions/workflows/ci.yml?query=branch%3Amain)
[![MIT License](https://img.shields.io/github/license/Leawind/dsh-desktop)](https://github.com/Leawind/dsh-desktop/blob/main/LICENSE)

<img src="docs/images/main.png" alt="DSH Desktop main workspace" width="80%">

</div>

---

## Why this app

- **Cross‑platform** – Works on Windows, Linux, and macOS with installer and portable releases.
- **Lightweight distribution** – The `slim` variant is only ~7–10 MB, ideal if you already have a DSH environment or only need to connect to a service.
- **Flexible DSH sources** – Choose the built‑in runtime, system `dsh`, a custom executable, npx, or directly connect to an existing DSH Web service.

## Which variant fits you

| Your situation                                                                 | Choose    | Package size | Notes                                                       |
| ------------------------------------------------------------------------------ | --------- | ------------ | ----------------------------------------------------------- |
| Want a ready-to-run app, or need both built‑in and external DSH launch options | `bundled` | 100–120 MB   | Full functionality, includes Node.js, DSH, and pnpm         |
| Already have DSH installed and don't need the bundled runtime                  | `slim`    | 7–10 MB      | All desktop management features except the built‑in runtime |

## Installation & first run

Download the release file matching your platform and variant. Windows provides MSI, macOS provides DMG, and Debian/Ubuntu provides DEB. Portable releases are also available as ZIP on Windows and macOS, or `tar.gz` on Linux. Installers and portable releases are updated by downloading and installing or extracting a newer release file.

### Linux runtime dependencies

Both Linux DEB and portable releases use the system GTK 3 and xdotool libraries and require a supported graphical browser. On Debian/Ubuntu, install the following packages; APT installs the transitive GTK, GDK, GLib, Cairo, X11/Wayland, and related libraries automatically:

```sh
sudo apt install libgtk-3-0 libxdo3
```

These are runtime requirements, not development requirements: end users do not need `libgtk-3-dev`, `libxdo-dev`, or `libappindicator3-dev`. Without an available browser, the app attempts to use the system WebView; it cannot display its interface when neither is available.

- Use the built‑in runtime (available in `bundled`);
- Use `dsh` from your system `PATH`;
- Pick a custom DSH executable or startup script;
- Launch via npx with `@deepseek-ai/dsh`;
- Enter the URL of an existing DSH Web service.

`bundled` defaults to the built‑in runtime, so no Node.js, npm, pnpm, or `dsh` is required upfront – but you can switch to any other method. `slim` has no built‑in runtime, but all other launch and connection options work identically.

If you choose the system DSH, install it via npm:

```sh
npm install -g @deepseek-ai/dsh
```

When using npx, Node.js must be installed; the first run will download the selected DSH version via npm.

## Interface overview

<div align="center">

<img src="docs/images/settings-dsh.png" width="94%">

Choose your DSH source and configure the DSH Home directory used by managed services.

---

<img src="docs/images/settings-startup.png" width="94%">

Define what happens when the application starts.

---

<img src="docs/images/settings-runtime.png" width="94%">

View and manage the built‑in runtime.

---

</div>

DeepSeek Harness is still in early preview – refer to its [official documentation](https://github.com/deepseek-ai/deepseek-harness/blob/master/README.zh.md) for installation and configuration details.

> [!IMPORTANT]
>
> This project is community‑developed and is not an official DeepSeek project, nor is it endorsed by DeepSeek.

[GitHub Releases]: https://github.com/Leawind/dsh-desktop/releases
