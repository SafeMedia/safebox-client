# SafeBox Client

<img width="1100" height="602" alt="local-client" src="https://github.com/user-attachments/assets/7928f719-f8d8-486f-9afe-2a55369cb978" />

## Getting Started

📚 Instructions on how to use the application can be found at: [Documentation](https://safemedia.com)

Visit the latest releases page:  
👉 [releases](https://github.com/SafeMedia/safebox-client/releases)

---

## Other Products

https://github.com/SafeMedia/safebox-server

https://github.com/SafeMedia/autonomi-chrome-extension

---

## Windows

Download either the `.exe` or `.msi` file.

When downloaded, double click it & enjoy.

---

## macOS

Until the application is added to the App Store, you can directly download the `.dmg` file.

Download the `.dmg` file.

Navigate to the directory containing the `.dmg` file using the `cd` command:

```bash
cd ~/Downloads
```

Then run this command from a terminal in the same folder as the .dmg file (be sure to update version number in command if newer version is released):

```bash
xattr -d com.apple.quarantine ./SafeBoxClient_0.1.6_aarch64.dmg
```

---

## Linux

Download the `.deb` file.

Navigate to the directory containing the `.deb` file using the `cd` command:

```bash
cd ~/Downloads
```

Then run this command from a terminal in the same folder as the .deb file (be sure to update version number in command if newer version is released):

```bash
sudo apt install ./SafeBoxClient_0.1.3_amd64.deb
```

---

## Developers

If you want to build the application locally for development purposes, follow the steps below:

### Prerequisites

System libraries and Rust:

[Tauri_v2](<[https://safemedia.com](https://v2.tauri.app/start/prerequisites/)>)

Node.js with yarn or npm

### Install Node Modules

```bash
yarn install
```

### Build Tauri App

```bash
yarn run tauri build
```

### Run Tauri App

```bash
yarn run tauri dev
```

### Icon Generation

To re-generate the icons for the project, run:

```bash
cargo tauri icon public/icon-gen.png
```


---

Make/find an issue & create a PR - All contributions appreciated!

## Contribute

Make/find an issue & create a PR - All contributions appreciated!
