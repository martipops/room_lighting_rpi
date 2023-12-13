use std::sync::mpsc;
use std::{thread, vec};
use warp::Filter;

mod led_driver;
mod twinkler;

pub fn control_leds(rx: mpsc::Receiver<serde_json::Value>) {

    let mut driver = led_driver::LEDDriver::new();
    driver.create_ping_pong_gradient();
    loop {
        if let Ok(body) = rx.try_recv() {
            driver.parse_change_message(body);
        }
        driver.do_animation_cycle();
    }
}


#[tokio::main]

async fn main() {
    let (tx, rx) = mpsc::channel();

    let cors = warp::cors()
        .allow_any_origin() // This allows requests from any domain. Be cautious with this setting in a production environment.
        .allow_headers(vec!["Content-Type"]) // Specify the headers you want to allow
        .allow_methods(vec!["POST"]); // Specify the methods you want to allow (e.g., GET, POST)

    let cors_clone = cors.clone();

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
        })
        .with(cors); // Apply the CORS filter here

    let static_files = warp::path("static")
        .and(warp::fs::dir("static"))
        .with(cors_clone);

    // Combine static files route with existing routes
    let routes = post_route.or(static_files);

    tokio::spawn(async move {
        warp::serve(routes).run(([0, 0, 0, 0], 5000)).await;
    });

    let led_thread = thread::spawn(move || control_leds(rx));

    led_thread.join().unwrap();
}
