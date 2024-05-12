use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, RwLock},
};

use chrono::Duration;
use plotters::{
    prelude::*,
    style::full_palette::{CYAN_300, CYAN_600, GREY},
};
use plotters_canvas::CanvasBackend;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use yew::prelude::*;
use yew_hooks::prelude::*;

use crate::data::{self, Kline, Time};

#[function_component(Chart)]
pub fn chart() -> Html {
    let canvas_container_ref: NodeRef = use_node_ref();
    let canvas_container_size = use_size(canvas_container_ref.clone());

    let data_ctx = use_context::<data::DataContext>().unwrap();
    let klines_len = data_ctx.klines.to_owned().read().unwrap().len();
    let canvas_ref = use_node_ref();
    let canvas = use_state_eq(|| None);

    let draw = {
        let canvas = canvas.clone();
        let klines = data_ctx.klines.clone();
        let heatmap = data_ctx.heatmap.clone();
        move || {
            if let Some(canvas) = canvas.as_ref() as Option<&HtmlCanvasElement> {
                // TODO: ThemeContext
                let local_storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
                let stored_theme = local_storage.get_item("color-theme").unwrap();
                let is_dark = stored_theme.unwrap_or("dark".to_string()) == "dark";

                let _ = redraw(canvas.clone(), is_dark, klines, heatmap);
            }
        }
    };
    // draw initial
    if (*canvas).is_none() {
        if let Some(canvas_el) = canvas_ref.cast::<HtmlCanvasElement>() {
            draw.clone()();
            canvas.set(Some(canvas_el));
        }
    }
    // redraw
    let state = use_state(|| 0);
    {
        let state = state.clone();
        use_interval(
            move || {
                state.set(*state + 1);
            },
            2000,
        );
    }
    {
        use_effect_with((canvas_container_size, state), move |_| {
            draw.clone()();
            || ()
        });
    }

    html! {
        <div ref={canvas_container_ref} class="w-full h-full overflow-hidden">
            <canvas
                ref={canvas_ref.clone()}
                width={canvas_container_size.0.to_string()}
                height={canvas_container_size.1.to_string()}
            ></canvas>
            <span class="absolute top-0 left-0">{klines_len}</span>
        </div>
    }
}

fn redraw(
    canvas: HtmlCanvasElement,
    darkmode: bool,
    klines: Arc<RwLock<BTreeMap<Time, Kline>>>,
    heatmap: Arc<RwLock<HashMap<Decimal, BTreeMap<Time, f64>>>>,
) -> anyhow::Result<()> {
    web_sys::console::log_1(&"redraw".into());

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();
    let rect = canvas.get_bounding_client_rect();
    context.clear_rect(0.0, 0.0, rect.width(), rect.height());

    let backend = CanvasBackend::with_canvas_object(canvas).expect("cannot find canvas");
    let root = backend.into_drawing_area();

    let klines = klines.read().unwrap();
    if klines.is_empty() {
        return Ok(());
    }
    let min_ts = klines.keys().min().unwrap();
    let min_ts = chrono::DateTime::from_timestamp(*min_ts as i64 / 1000, 0).unwrap();
    let max_ts = klines.keys().max().unwrap();
    let max_ts = chrono::DateTime::from_timestamp(*max_ts as i64 / 1000, 0).unwrap();
    let min_px = klines
        .values()
        .map(|k| k.lo)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_px = klines
        .values()
        .map(|k| k.hi)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let min_max_px_spd = (max_px - min_px) / 2.0;
    let min_px = min_px - min_max_px_spd;
    let max_px = max_px + min_max_px_spd;

    let mut chart = ChartBuilder::on(&root)
        .margin(10u32)
        .x_label_area_size(30u32)
        .y_label_area_size(30u32)
        .build_cartesian_2d(min_ts..max_ts, min_px..max_px)?;

    let axis_color = if darkmode { GREY } else { BLACK };
    let heatmap_color = if darkmode { CYAN_600 } else { CYAN_300 };
    chart
        .configure_mesh()
        .disable_mesh()
        .bold_line_style(axis_color.mix(0.02))
        .light_line_style(axis_color.mix(0.05))
        .axis_style(ShapeStyle::from(axis_color.mix(0.45)).stroke_width(1))
        .y_labels(10)
        .y_label_style(
            ("monospace", 12)
                .into_font()
                .color(&axis_color.mix(0.65))
                .transform(FontTransform::Rotate90),
        )
        .y_label_formatter(&|y| format!("{}", (*y * 10_000.0).round() / 10_000.0))
        .x_labels(8)
        .x_label_style(("monospace", 12).into_font().color(&axis_color.mix(0.65)))
        .x_label_formatter(&|x| x.format("%H:%M:%S").to_string())
        .draw()?;

    // Draw heatmap
    if let Ok(heatmap) = heatmap.read() {
        let sizes: Vec<_> = heatmap
            .values()
            .flat_map(|m| m.values()) // Accessing all f64 values across all BTreeMaps
            .map(|&size| size.abs()) // Apply abs() to each f64 value
            .filter(|&v| v.is_finite() && v != 0.0) // Filter values based on the conditions
            .collect();
        // let max_sz = sizes.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let median_sz = data::median(&sizes);
        let deviations: Vec<_> = sizes.iter().map(|&x| (x - median_sz).abs()).collect();
        let mdev_sz = data::median(&deviations);

        for (price, orders) in heatmap.iter() {
            let mut points = Vec::new();
            let price_coord = price.to_f64().unwrap();
            if price_coord <= min_px || price_coord >= max_px {
                continue;
            }
            let mut last_sz = 0.0f64;

            for (&time, &size) in orders.iter() {
                let size_coord = size.abs(); // Convert size to z-coordinate
                let time_coord = chrono::DateTime::from_timestamp(time as i64 / 1000, 0).unwrap(); // Convert time to x-coordinate
                if time_coord < min_ts || time_coord > max_ts {
                    continue;
                }
                points.push((time_coord, price_coord)); // Add point to line

                // Draw line segment if there are at least two points
                if points.len() >= 2 {
                    let line_width;
                    let alpha_scale;
                    let start_size = last_sz.abs().max(size_coord);
                    let threshold = (start_size - median_sz) / mdev_sz;
                    // console::log_1(&format!("size: {:?} threshold: {:?}", start_size, threshold).into());
                    if threshold > 9.0 {
                        line_width = 8.0;
                        alpha_scale = 1.0;
                    } else {
                        line_width = 4.0;
                        alpha_scale = 0.3 * threshold.max(0.1).min(1.0);
                    }
                    let line_style = heatmap_color
                        .mix(alpha_scale)
                        .stroke_width(line_width as u32);
                    // if size < 0.0 || last_sz < 0.0 {
                    //     line_style = plotters::style::RGBAColor(255, 0, 255, alpha_scale)
                    //         .stroke_width(line_width as u32);
                    // }
                    chart
                        .draw_series(LineSeries::new(points.iter().cloned(), line_style))
                        .unwrap(); // Draw line

                    points.clear();

                    if size_coord > 0.0 {
                        points.push((time_coord - Duration::seconds(1), price_coord));
                    }
                }

                last_sz = size;
            }

            if points.len() > 0 {
                points.push((chrono::Utc::now(), price_coord));
                let line_width;
                let alpha_scale;
                let start_size = last_sz.abs();
                let threshold = (start_size - median_sz) / mdev_sz;
                if threshold > 9.0 {
                    line_width = 8.0;
                    alpha_scale = 1.0;
                } else {
                    line_width = 4.0;
                    alpha_scale = 0.3 / threshold.max(1.0);
                }
                let line_style = heatmap_color
                    .mix(alpha_scale)
                    .stroke_width(line_width as u32)
                    .filled();
                // if last_sz < 0.0 {
                //     line_style = plotters::style::RGBAColor(255, 0, 255, alpha_scale)
                //         .stroke_width(line_width as u32);
                // }
                chart
                    .draw_series(LineSeries::new(points.iter().cloned(), line_style))
                    .unwrap(); // Draw line
                points.clear();
            }
        }
        drop(heatmap);
    }

    // Draw KLines
    chart.draw_series(klines.values().map(|k| {
        CandleStick::new(
            chrono::DateTime::from_timestamp(k.ts as i64 / 1000, 0).unwrap(),
            k.op,
            k.hi,
            k.lo,
            k.cl,
            RGBColor(81, 205, 160).filled(),
            RGBColor(192, 80, 77).filled(),
            8,
        )
    }))?;

    root.present()?;

    Ok(())
}
