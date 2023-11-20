use std::sync::mpsc;
use std::{thread, vec};
use warp::Filter;

use rs_ws281x::ChannelBuilder;
use rs_ws281x::ControllerBuilder;
use rs_ws281x::StripType;

#[derive(Debug)]

enum ColorMode {
    Off,
    Gradient,
    GradientCenter,
    Twinkle,
    Rainbow,
}

#[derive(Debug)]
struct AppState {
    mode: ColorMode,
    colors: Vec<[u8; 4]>,
    brightness: u8,
    frequency: u8,
    speed: u8,
}

fn parse_change_message(state: &mut AppState, body: serde_json::Value) {
    if let Some(new_colors) = body["colors"].as_str() {
        let vectors = new_colors
            .trim_matches(|c| c == '(' || c == ')')
            .split("),(")
            .filter_map(|s| {
                let mut nums = s.split(',').filter_map(|num| num.parse::<u8>().ok());
                Some([nums.next()?, nums.next()?, nums.next()?, nums.next()?])
            })
            .collect::<Vec<[u8; 4]>>();

        state.colors = vectors;
    }
    if let Some(new_mode) = body["mode"].as_str() {
        state.mode = match new_mode.to_string().to_lowercase().as_str() {
            "gradient" => ColorMode::Gradient,
            "gradientCenter" => ColorMode::GradientCenter,
            "off" => ColorMode::GradientCenter,
            _ => ColorMode::Off,
        };
    }

    //TODO finish all cases of the struct body parse
}

fn control_leds(rx: mpsc::Receiver<serde_json::Value>) {
    let mut controller = ControllerBuilder::new()
        .freq(800_000)
        .dma(0)
        .channel(
            0, // Channel Index
            ChannelBuilder::new()
                .pin(12) // GPIO 10 = SPI0 MOSI
                .count(460) // Number of LEDs
                .strip_type(StripType::Ws2812)
                .brightness(255) // default: 255
                .build(),
        )
        .build()
        .unwrap();

    let mut state = AppState {
        mode: ColorMode::GradientCenter,
        colors: vec![[255, 0, 0, 0], [0, 0, 255, 0]],
        brightness: 0,
        frequency: 1,
        speed: 1,
    };

    let mut j = 0;
    loop {
        if let Ok(body) = rx.try_recv() {
            parse_change_message(&mut state, body);
        }

        // TODO: create a function to do one iteration of a led animation based on the enum values
        let c = &state.colors;
        for (i, led) in controller.leds_mut(0).into_iter().enumerate() {
            *led = c[j];
        }
        j += 1;
        j %= 2;
        controller.render().unwrap();
        thread::sleep(std::time::Duration::from_secs(2));
        //END TODO
    }
}

#[tokio::main]
async fn main() {
    let (tx, rx) = mpsc::channel();

    let post_route = warp::post()
        .and(warp::path("update"))
        .and(warp::body::json())
        .map(move |body: serde_json::Value| {
            if tx.send(body).is_err() {
                eprintln!("Failed to send state to LED control thread.");
                return warp::reply::json(&serde_json::json!({"status": "error"}));
            }

            println!("State updated");
            warp::reply::json(&serde_json::json!({"status": "success"}))
        });

    tokio::spawn(async move {
        warp::serve(post_route).run(([0, 0, 0, 0], 3030)).await;
    });

    let led_thread = thread::spawn(move || control_leds(rx));

    led_thread.join().unwrap();
}
