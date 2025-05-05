# SafeBox Client

![SafeBox Client](https://github.com/SafeMedia/images/blob/main/impossible_futures/safebox/safebox-client.png)

## Getting Started

### Developers

-   System libraries and Rust: https://v2.tauri.app/start/prerequisites/

-   Download Rust dependencies.

### Frontend

1. Node.js and yarn/npm

1. **Install Node modules**:  
   Run the following command in your terminal:

    ```bash
    yarn install

    ```

1. **Build Tauri App**:  
   Run the following command in your terminal:

    ```bash
    yarn run tauri build

    ```

1. **Run Tauri App**:  
   Run the following command in your terminal:
    ```bash
    yarn run tauri dev
    ```

Side-note: If you wish to run the development web server (if you for some reason don't want to run it via tauri):

```bash
yarn run dev

```

### Icon Generation

To re-generate the icons for the project you can run:

```
cargo tauri icon public/icon-gen.png
```

### WARNING

SafeBox client is in active development, please only use testnet funds, or if using mainnet, small amounts!

### Testnet

-   If you want to connect to local testnet:

Edit `await invoke("connect", [...]` line in `connect()` function in `src/backend/autonomi.tsx` file, and replace `peer` value with one of your local nodes' Multiaddress, which can be found in node logs. You can see where logs are stored when starting local network, the startup process will output something like "Logging to directory: [...]". In the logfile you search for the line looking like this: `Local node is listening ListenerId(1) on "/ip4/127.0.0.1/udp/11111/quic-v1/p2p/AaaAaa11AaaAaa11AaaAaa11AaaAaa11AaaAaa11AaaAaa11AaaA"`. Address can also be checked running this command:

`cargo run --release --bin safenode-manager --features local -- local status --details`

-   (alternatively) Connecting to official testnet:

Same as above, but instead local node's Multiaddr in `peer`, just insert something that is not Multiaddr, or leave it empty.


### Contribute

Make/find an issue & create a PR - All contributions appreciated!
