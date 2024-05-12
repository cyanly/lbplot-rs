use console_error_panic_hook::set_once as set_panic_hook;
use yew::prelude::*;
mod chart;
mod data;
mod theme_switch;
mod tickers;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[function_component]
pub fn App() -> Html {
    let links = [
        ("Github", "https://github.com/cyanly/gotrade"),
        ("Blog", "https://cyan.ly/blog"),
    ];

    let data_ctx = use_reducer(data::Data::default);
    let symbol = use_state(|| "BTCUSDT".to_string());
    {
        let data_ref = data_ctx.dispatcher().clone();
        use_effect_with(symbol.clone(), move |_| {
            data_ref.dispatch(data::DataAction::SetSymbol(symbol.to_string()));
        });
    }

    html! {
        <div class="min-h-screen bg-gray-50 dark:bg-gray-900 text-black dark:text-white">
            <nav class="w-full h-16 py-2 bg-gray-100 dark:bg-gray-950">
                <div class="container flex mx-auto gap-6 items-center h-full">
                    <h1 class="font-bold text-2xl text-black dark:text-white">{"OrderBook Visualisation Demo"}</h1>
                    <div class="flex-1"></div>
                    {for links.iter().map(|(label, href)| html! {
                        <a target="_blank" class="block px-4 py-2 hover:bg-black hover:text-white dark:text-white dark:bg-indigo-500 dark:hover:bg-white dark:hover:text-black rounded border-black border" href={*href}>{label}</a>
                    })}
                    <theme_switch::Button/>
                </div>
            </nav>

            <ContextProvider<data::DataContext> context={data_ctx}>
            <div class="flex flex-row flex-grow h-[calc(100vh-64px)]">
                <div class="w-1/5 min-w-[200px] h-full overflow-auto bg-gray-100 dark:bg-gray-950">
                   <tickers::TickerProvider>
                   <tickers::TickerList/>
                   </tickers::TickerProvider>
                </div>

                <div class="flex-grow h-full overflow-auto">
                    <chart::Chart/>
                </div>
            </div>
            </ContextProvider<data::DataContext>>
        </div>
    }
}

fn main() {
    set_panic_hook();

    yew::Renderer::<App>::new().render();
}
