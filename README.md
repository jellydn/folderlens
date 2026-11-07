# Welcome to Shelf 👋

![Version](https://img.shields.io/badge/version-0.1.0-blue.svg?cacheSeconds=2592000)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/jellydn/shelf/blob/main/LICENSE)
[![Twitter: jellydn](https://img.shields.io/twitter/follow/jellydn.svg?style=social)](https://twitter.com/jellydn)

> A macOS menu-bar review of what a cleanup would free. Confirm records the choice and deletes nothing.

## Screenshot

The panel opens from the menu bar.

![Shelf panel](shots/tray.png)

Protected folders, the hour hold, and the locked system paths.

![Protected settings](shots/settings-protected.png)

Agents that are on `PATH`.

![Agent settings](shots/settings-agent.png)

## Features

- Menu-bar panel with Memory, CPU, and GPU
- Free space, ready space, and a capacity bar
- Careful, Balanced, and Aggressive choices, plus a Selected override
- One summary line for what the choice would free
- Actions for the row under the pointer: Finder, keep, unlock, ask, and the rest
- Protected folders you add, a “keep what changed within” hold, and built-in locked paths
- AI tools detected on `PATH`, from the same command list as [ai-launcher](https://github.com/jellydn/ai-launcher)
- Ask starts the chosen tool only after you grant access
- Settings and the panel are never open together

## Prerequisites

- macOS
- Rust, pinned in `rust-toolchain.toml`

## Install

```sh
git clone https://github.com/jellydn/shelf
cd shelf
cargo run --release
```

The archivebox icon appears in the menu bar after the first scan. Click it to open the panel. Click it again to close the panel.

Open Settings from Actions, or with Command-,. The panel closes. Click the menu-bar icon to close Settings and open the panel.

## Run tests

```sh
cargo test
```

A headless picture of the panel:

```sh
cargo run -- --theme dark --snap shots/tray.png
```

## Author

👤 **Huynh Duc Dung**

- Website: https://productsway.com/
- Twitter: [@jellydn](https://twitter.com/jellydn)
- Github: [@jellydn](https://github.com/jellydn)

## Show your support

[![kofi](https://img.shields.io/badge/Ko--fi-F16061?style=for-the-badge&logo=ko-fi&logoColor=white)](https://ko-fi.com/dunghd)
[![paypal](https://img.shields.io/badge/PayPal-00457C?style=for-the-badge&logo=paypal&logoColor=white)](https://paypal.me/dunghd)
[![buymeacoffee](https://img.shields.io/badge/Buy_Me_A_Coffee-FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black)](https://www.buymeacoffee.com/dunghd)

Give a ⭐️ if this project helped you!

[![Stargazers repo roster for @jellydn/shelf](https://reporoster.com/stars/jellydn/shelf)](https://github.com/jellydn/shelf/stargazers)
