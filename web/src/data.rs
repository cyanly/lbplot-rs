use std::collections::{BTreeMap, HashMap, VecDeque};
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};

use rust_decimal::prelude::*;
use serde_json::Value;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{console, js_sys, MessageEvent, WebSocket};
use yew::prelude::*;

pub type Time = u64;

//MARK: - Context Interfaces ---------------------------------------------

#[derive(Clone)]
pub struct Data {
    pub symbol: Option<String>,
    pub klines: Arc<RwLock<BTreeMap<Time, Kline>>>,
    pub updates: Arc<Mutex<VecDeque<(Time, OrderBookUpdate)>>>,
    pub heatmap: Arc<RwLock<HashMap<Decimal, BTreeMap<Time, f64>>>>,

    ws: Option<WebSocket>,
    timer_handle: Option<i32>,
}
impl PartialEq for Data {
    fn eq(&self, other: &Self) -> bool {
        self.symbol == other.symbol && self.ws == other.ws
    }
}
impl Default for Data {
    fn default() -> Self {
        Self {
            symbol: None,
            klines: Arc::new(RwLock::new(BTreeMap::new())),
            updates: Arc::new(Mutex::new(VecDeque::new())),
            heatmap: Arc::new(RwLock::new(HashMap::new())),

            ws: None,
            timer_handle: None,
        }
    }
}

impl Reducible for Data {
    type Action = DataAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let new_state = self.deref().clone();
        match action {
            DataAction::SetSymbol(symbol) => {
                let mut data_ref = self.deref().clone();
                yew::platform::spawn_local(async move {
                    data_ref.set_symbol(symbol).await;
                });
            }
        };
        new_state.into()
    }
}
#[derive(Clone)]
pub enum DataAction {
    SetSymbol(String),
}
pub type DataContext = UseReducerHandle<Data>;

//MARK: - Data Structures ---------------------------------------------

#[derive(Debug, Clone)]
pub struct OrderBookUpdate {
    pub ts: Time, // timestamp
    pub sq: u64,  // sequence
    pub px: f64,  // price
    pub sz: f64,  // size, (-ve for offers)
}

#[derive(Debug, Clone)]
pub struct Kline {
    pub ts: Time, // open time
    pub op: f64,
    pub hi: f64,
    pub lo: f64,
    pub cl: f64,
    pub vb: f64,  // volume buy
    pub vs: f64,  // volume sell
    pub tc: Time, // close time
}

//MARK: - Data Provider ---------------------------------------------

impl Data {
    pub async fn set_symbol(&mut self, symbol: String) {
        if let Some(sym) = &self.symbol {
            if *sym == symbol {
                return;
            }
        }
        self.symbol = Some(symbol);
        self.clear();

        console::log_1(&format!("[data] set_symbol {:?}", self.symbol).into());
        self.dial().await;
    }

    fn clear(&mut self) {
        self.klines.write().unwrap().clear();
        self.updates.lock().unwrap().clear();
        self.heatmap.write().unwrap().clear();

        if let Some(handle) = self.timer_handle {
            let window = web_sys::window().expect("should have a window in this context");
            window.clear_timeout_with_handle(handle);
            self.timer_handle = None;
        }
    }

    async fn dial(&mut self) {
        if let Some(ws) = &self.ws {
            ws.close().unwrap();
            self.clear();
        }

        let symbol = self.symbol.as_ref().unwrap().to_lowercase();
        let url = "wss://data-stream.binance.vision/stream";
        let ws = WebSocket::new(&format!(
            "{}?streams={}@depth@100ms/{}@kline_1s",
            url, symbol, symbol
        ))
        .unwrap();

        {
            let updates = self.updates.clone();
            let klines = self.klines.clone();

            let on_msg = Closure::wrap(Box::new(move |event: MessageEvent| {
                if let Ok(data) = event.data().dyn_into::<js_sys::JsString>() {
                    let data_str: String = data.into();
                    let msg: Value = serde_json::from_str(&data_str).unwrap();
                    match msg["stream"].as_str() {
                        Some(stream) if stream.contains("kline") => {
                            // console::log_1(&format!("📈 [ws] kline {:?}", msg).into());
                            if let Some(kline) = msg["data"]["k"].as_object() {
                                let ts = kline["t"].as_u64().unwrap();
                                let op = kline["o"].as_str().unwrap().parse::<f64>().unwrap();
                                let hi = kline["h"].as_str().unwrap().parse::<f64>().unwrap();
                                let lo = kline["l"].as_str().unwrap().parse::<f64>().unwrap();
                                let cl = kline["c"].as_str().unwrap().parse::<f64>().unwrap();
                                let vb = kline["V"].as_str().unwrap().parse::<f64>().unwrap();
                                let vo = kline["v"].as_str().unwrap().parse::<f64>().unwrap();
                                let tc = kline["T"].as_u64().unwrap();
                                if let Some(_) = klines.write().unwrap().insert(
                                    ts,
                                    Kline {
                                        ts,
                                        op,
                                        hi,
                                        lo,
                                        cl,
                                        vb,
                                        vs: vo - vb,
                                        tc,
                                    },
                                ) {
                                } else {
                                    // let heatmap = heapmap.read().unwrap();
                                    // let formatted_events: String = heatmap
                                    //     .iter()
                                    //     .map(|(price, time_sz_map)| {
                                    //         let price_str = format!("Price: {:?}", price);
                                    //         let time_sz_str: String = time_sz_map
                                    //             .iter()
                                    //             .map(|(time, sz)| {
                                    //                 format!("  Time: {}, Size: {}", *time, *sz)
                                    //             })
                                    //             .collect::<Vec<String>>()
                                    //             .join("\n");
                                    //         format!("{}\n{}", price_str, time_sz_str)
                                    //     })
                                    //     .collect::<Vec<String>>()
                                    //     .join("\n\n");
                                    // drop(heatmap);
                                    // console::log_1(&formatted_events.into());
                                }
                            }
                        }
                        Some(stream) if stream.contains("depth") => {
                            // console::log_1(&format!("📊 [ws] depth {:?}", msg).into());
                            let mut updates = updates.lock().unwrap();
                            // let ts = chrono::Utc::now().timestamp_millis() as u64;
                            // let sq = msg["data"]["lastUpdateId"].as_u64().unwrap();
                            // if let Some(v) = msg["data"]["bids"].as_array() {
                            //     for bid in v {
                            //         let px = bid[0].as_str().unwrap().parse::<f64>().unwrap();
                            //         let sz = bid[1].as_str().unwrap().parse::<f64>().unwrap();
                            //         updates.push_back((ts, OrderBookUpdate { ts, sq, px, sz }));
                            //     }
                            // }
                            // if let Some(v) = msg["data"]["asks"].as_array() {
                            //     for ask in v {
                            //         let px = ask[0].as_str().unwrap().parse::<f64>().unwrap();
                            //         let sz = ask[1].as_str().unwrap().parse::<f64>().unwrap();
                            //         updates.push_back((
                            //             ts,
                            //             OrderBookUpdate {
                            //                 ts,
                            //                 sq,
                            //                 px,
                            //                 sz: -sz,
                            //             },
                            //         ));
                            //     }
                            // }
                            let ts = msg["data"]["E"].as_u64().unwrap();
                            let sq = msg["data"]["u"].as_u64().unwrap();
                            if let Some(v) = msg["data"]["b"].as_array() {
                                for bid in v {
                                    let px = bid[0].as_str().unwrap().parse::<f64>().unwrap();
                                    let sz = bid[1].as_str().unwrap().parse::<f64>().unwrap();
                                    updates.push_back((ts, OrderBookUpdate { ts, sq, px, sz }));
                                }
                            }
                            if let Some(v) = msg["data"]["a"].as_array() {
                                for ask in v {
                                    let px = ask[0].as_str().unwrap().parse::<f64>().unwrap();
                                    let sz = ask[1].as_str().unwrap().parse::<f64>().unwrap();
                                    updates.push_back((
                                        ts,
                                        OrderBookUpdate {
                                            ts,
                                            sq,
                                            px,
                                            sz: -sz,
                                        },
                                    ));
                                }
                            }
                        }
                        _ => {
                            console::log_1(&format!("🚫 [ws] stream {:?}", msg).into());
                        }
                    }
                }
            }) as Box<dyn FnMut(MessageEvent)>);
            ws.set_onmessage(Some(on_msg.as_ref().unchecked_ref()));
            on_msg.forget();
        }

        self.schedule_processing();
        self.ws = Some(ws);
    }

    fn schedule_processing(&mut self) {
        let updates = self.updates.clone();
        let heatmap = self.heatmap.clone();
        let closure = Closure::<dyn Fn()>::new(Box::new(move || {
            let mut queue = updates.lock().unwrap();
            if !queue.is_empty() {
                let mut heatmap = heatmap.write().unwrap();
                Data::process_updates(&mut *heatmap, &mut *queue);
                drop(heatmap);
            }
        }));

        let window = web_sys::window().expect("should have a window in this context");
        let handle = window
            .set_interval_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                300, // Trigger after 1000 milliseconds
            )
            .unwrap();
        closure.forget();
        self.timer_handle = Some(handle);
    }

    fn process_updates(
        heatmap: &mut HashMap<Decimal, BTreeMap<Time, f64>>,
        updates: &mut VecDeque<(u64, OrderBookUpdate)>,
    ) {
        let price_step = 1.0;
        let time_step = 1000;

        while let Some(update) = updates.pop_front() {
            // console::log_1(&format!("update {:?}", update).into());
            let price_bin = (update.1.px / price_step).floor() * price_step;
            let time_bin = (update.0 / time_step) * time_step;
            // console::log_1(&format!("price {:?} time {:?}", price_bin, time_bin).into());

            let bin = heatmap
                .entry(Decimal::from_f64(price_bin).unwrap())
                .or_insert_with(BTreeMap::new);

            if let Some((&last_time_bin, &last_size)) = bin.iter().last() {
                if last_size == update.1.sz && !last_size.is_zero() {
                    continue;
                }
                if last_time_bin == time_bin && !update.1.sz.is_zero() {
                    *bin.get_mut(&last_time_bin).unwrap() = update.1.sz;
                } else {
                    bin.insert(time_bin, update.1.sz);
                }
            } else {
                // Insert new entry if the time bin is different
                bin.insert(time_bin, update.1.sz);
            };
        }

        if heatmap.len() > 200 {
            // Reject outliers
            let prices: Vec<_> = heatmap
                .keys()
                .into_iter()
                .map(|r| r.to_f64().unwrap())
                .collect();
            let m = median(&prices);
            let deviations: Vec<_> = prices.iter().map(|&x| (x - m).abs()).collect();
            let mdev = median(&deviations);
            let to_remove: Vec<_> = prices
                .iter()
                .filter(|&r| {
                    let price = r.to_f64().unwrap();
                    let dev = (price - m).abs();
                    dev / mdev > 2.0
                })
                .map(|r| Decimal::from_f64(*r).unwrap())
                .collect();
            console::log_1(&format!("outliers {:?}", to_remove.len()).into());
            for price in to_remove {
                heatmap.remove(&price);
            }
        }
    }
}

pub fn median(data: &[f64]) -> f64 {
    let mut sorted_data = data.to_vec();
    sorted_data.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = sorted_data.len() / 2;
    if sorted_data.len() % 2 == 0 {
        (sorted_data[mid - 1] + sorted_data[mid]) / 2.0
    } else {
        sorted_data[mid]
    }
}

#[allow(dead_code)]
fn histogram(data: &[f64], bins: usize) -> (Vec<usize>, Vec<f64>) {
    let min_value = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_value = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    console::log_1(&format!("min: {:?} max: {:?}", min_value, max_value).into());

    let bin_width = (max_value - min_value) / bins as f64;

    let mut bin_counts = vec![0; bins];
    let mut bin_boundaries = Vec::with_capacity(bins + 1);
    for i in 0..=bins {
        bin_boundaries.push(min_value + i as f64 * bin_width);
    }

    for &value in data {
        let mut bin_index = (value - min_value) / bin_width;
        // adjust bin_index to ensure it falls within the valid range
        bin_index = bin_index.max(0.0).min((bins - 1) as f64);
        let bin_index = bin_index as usize;
        bin_counts[bin_index] += 1;
    }

    (bin_counts, bin_boundaries)
}
