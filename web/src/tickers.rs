use std::rc::Rc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use web_sys::console;
use yew::prelude::*;
use yew_hooks::prelude::*;

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
pub struct Tick {
    pub symbol: String,
    pub best_bid: f64,
    pub best_ask: f64,
    pub volume: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TickerState {
    pub tickers: Vec<Tick>,
}

pub enum TickerActions {
    SetTickers(Vec<Tick>),
}

impl TickerState {
    pub fn new() -> Self {
        Self {
            tickers: Vec::<Tick>::default(),
        }
    }
}

impl Reducible for TickerState {
    type Action = TickerActions;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            TickerActions::SetTickers(tickers) => { TickerState { tickers } }.into(),
        }
    }
}

pub type TickerContext = UseReducerHandle<TickerState>;

#[derive(Properties, Debug, PartialEq)]
pub struct TickerProviderProps {
    #[prop_or_default]
    pub children: Children,
}

#[function_component(TickerProvider)]
pub fn ticker_provider(props: &TickerProviderProps) -> Html {
    let state = use_reducer(TickerState::new);

    let ws = use_websocket("wss://stream.binance.com:9443/stream?streams=!ticker@arr".to_string());
    {
        let state = state.clone();
        let ws = ws.clone();
        use_effect_update_with_deps(
            move |message| {
                if let Some(msg) = &**message {
                    let data_str: String = msg.into();
                    let msg: Value = serde_json::from_str(&data_str).unwrap();
                    // console::log_1(&format!("🦜 [ws] tick {:?}", msg).into());

                    match msg["stream"].as_str() {
                        Some(stream) if stream.contains("!ticker@arr") => {
                            if let Some(v) = msg["data"].as_array() {
                                let mut tickers = v
                                    .iter()
                                    .map(|t| Tick {
                                        symbol: t["s"].as_str().unwrap().to_string(),
                                        best_bid: t["b"].as_str().unwrap().parse().unwrap(),
                                        best_ask: t["a"].as_str().unwrap().parse().unwrap(),
                                        volume: t["q"].as_str().unwrap().parse().unwrap(),
                                    })
                                    .collect::<Vec<Tick>>();
                                tickers.sort_by(|a, b| b.volume.partial_cmp(&a.volume).unwrap());
                                // Taking the top 25 tickers
                                let top_tickers =
                                    tickers.iter().take(25).cloned().collect::<Vec<Tick>>();
                                console::log_1(
                                    &format!("🦜 [ws] tickers {:?}", top_tickers).into(),
                                );

                                state.dispatch(TickerActions::SetTickers(top_tickers));
                            }
                        }
                        _ => {
                            console::log_1(&format!("🚫 [ws] stream {:?}", msg).into());
                        }
                    }
                }
                || ()
            },
            ws.message,
        );
    }

    html! {
        <ContextProvider<TickerContext> context={state.clone()}>
            {props.children.clone()}
        </ContextProvider<TickerContext>>
    }
}

#[function_component(TickerList)]
pub fn ticker_list() -> Html {
    let context = use_context::<TickerContext>().unwrap();
    let tickers = &*context.tickers;
    let tickers = tickers.iter().map(|t| {
        html! {
            <div class="flex flex-row justify-between items-center p-2 border-b border-gray-200 dark:border-gray-800">
                <div class="flex flex-col">
                    <span class="text-sm font-bold">{&t.symbol}</span>
                    <span class="text-xs text-gray-500 dark:text-gray-400">{format!("{} / {}", t.best_bid, t.best_ask)}</span>
                </div>
            </div>
        }
    })
    .collect::<Html>();

    html! {
        <>
        {tickers}
        </>
    }
}
