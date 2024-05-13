# Limit OrderBook Chart with Heatmap


> This project demonstrate heatmap visualisation tool for Limit OrderBook. It combines order book data and heatmap visualization to offer unique insights into market dynamics. It attempts to visualise how much money is waiting to be executed on each price level. Making it easier for human to analyse fake liquidity and useful to assist reverse engineering other market participants algorithms. 

- [x] SPA web with No Javascript
- [x] No `node_modules` were used in the making of this web app.
- [x] Dark mode
- [ ] For demo purpose no server: no historical data load


### Made possible with

- 📦 Rust (wasm-bindgen + web_sys)
- 📦 Yew https://yew.rs/
- 📦 Yew Hooks https://jetli.github.io/yew-hooks/#/
- 📦 Plotters-rs https://github.com/plotters-rs/plotters
- 🏗 Trunk https://trunkrs.dev/
- 📈 Binance market data websocket

---


<img width="1725" alt="image" src="https://github.com/cyanly/lbplot-rs/assets/5181446/5605bca0-57bc-416f-a48e-d65fae58a00d">




## Live Demo

[https://cyan.ly/lbplot-rs/](https://cyan.ly/lbplot-rs/)

## Getting Started

### Prerequisites

- Rust
- Any modern web browser that supports WebAssembly.
- [Trunk - Build, bundle & ship your Rust WASM application to the web.](https://github.com/trunk-rs/trunk)

### Build and Run

1. **Trunk needs to be installed at path:**

   ```bash
   cargo binstall trunk
   ```

2. **Run in local browser:**
   ```bash
   cd web
   trunk serve --open
   ```

3. **Browser** `http://localhost:8080`

### License
> Distributed under the MIT License. See LICENSE for more information.
