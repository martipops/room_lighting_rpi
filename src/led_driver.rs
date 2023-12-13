use rs_ws281x::ChannelBuilder;
use rs_ws281x::Controller;
use rs_ws281x::ControllerBuilder;
use rs_ws281x::StripType;
use std::{thread, vec};

use crate::twinkler;

pub const LED_COUNT: usize = 460;
const LED_PIN: i32 = 12;
const STRIP_TYPE: StripType = StripType::Ws2812;

#[derive(Debug)]
enum ColorMode {
    Off,
    Gradient,
    GradientCenter,
    Solid,
    Twinkle,
}

#[derive(Debug)]
pub struct LEDDriver {
    mode: ColorMode,
    colors: Vec<[u8; 4]>,
    colormap: Vec<[u8; 4]>,
    colorsteps: usize,
    delay: u64,
    controller: Controller,
    twinkle_driver: twinkler::Twinkler
}

impl LEDDriver {
    pub fn new() -> Self {
        let controller = ControllerBuilder::new()
            .freq(800_000)
            .dma(0)
            .channel(
                0, // Channel Index
                ChannelBuilder::new()
                    .pin(LED_PIN)
                    .count(LED_COUNT as i32)
                    .strip_type(STRIP_TYPE)
                    .brightness(255)
                    .build(),
            )
            .build()
            .unwrap();

        LEDDriver {
            mode: ColorMode::Twinkle,
            colors: vec![[255, 0, 0, 0], [0, 0, 255, 0]],
            colormap: vec![[255, 0, 0, 0], [0, 0, 255, 0]],
            colorsteps: 255,
            delay: 2,
            controller: controller,
            twinkle_driver: twinkler::Twinkler::new()
        }
    }

    fn interpolate_segment(
        start_color: [u8; 4],
        end_color: [u8; 4],
        segment_steps: usize,
    ) -> Vec<[u8; 4]> {
        fn interpolate(start: u8, end: u8, ratio: f32) -> u8 {
            (start as f32 + (end as f32 - start as f32) * ratio).round() as u8
        }
        let mut segment = Vec::new();
        for step in 0..segment_steps {
            let ratio = step as f32 / segment_steps as f32;
            let color = [
                interpolate(start_color[0], end_color[0], ratio),
                interpolate(start_color[1], end_color[1], ratio),
                interpolate(start_color[2], end_color[2], ratio),
                interpolate(start_color[3], end_color[3], ratio),
            ];
            segment.push(color);
        }
        segment
    }

    pub fn create_ping_pong_gradient(&mut self) {
        let mut gradient = Vec::new();
        let segment_steps = self.colorsteps / ((self.colors.len() - 1) * 2);

        for pass in 0..2 {
            for i in 0..self.colors.len() - 1 {
                let (start_color, end_color) = if pass == 0 {
                    (self.colors[i], self.colors[i + 1])
                } else {
                    (
                        self.colors[self.colors.len() - 1 - i],
                        self.colors[self.colors.len() - 2 - i],
                    )
                };

                gradient.extend(Self::interpolate_segment(
                    start_color,
                    end_color,
                    segment_steps,
                ));
            }
        }
        self.colormap = gradient;
    }

    pub fn parse_change_message(&mut self, body: serde_json::Value) {
        if let Some(new_colors) = body["colors"].as_str() {
            let vectors = new_colors
                .trim_matches(|c| c == '(' || c == ')')
                .split("),(")
                .filter_map(|s| {
                    let nums: Vec<u8> = s
                        .split(',')
                        .filter_map(|num| num.parse::<u8>().ok())
                        .collect();

                    // Check if we have exactly three color components (R, G, B)
                    if nums.len() == 3 {
                        // Rearrange from RGB to BRG
                        let brg = [nums[2], nums[1], nums[0], 255]; // Assuming alpha value as 255
                        Some(brg)
                    } else {
                        Some([0, 0, 0, 255])
                    }
                })
                .collect::<Vec<[u8; 4]>>();

            self.colors = vectors;
        }

        if let Some(new_mode) = body["mode"].as_str() {
            self.mode = match new_mode.to_string().to_lowercase().as_str() {
                "gradient" => ColorMode::Gradient,
                "gradientcenter" => ColorMode::GradientCenter,
                "off" => ColorMode::Off,
                "solid" => ColorMode::Solid,
                "twinkle" => ColorMode::Twinkle,
                _ => ColorMode::Off,
            };
        }

        if let Some(new_brightness) = body["brightness"].as_i64() {
            self.controller.set_brightness(0, new_brightness as u8)
        }

        if let Some(new_colorsteps) = body["colorsteps"].as_i64() {
            self.colorsteps = new_colorsteps as usize;
        }

        if let Some(new_delay) = body["delay"].as_i64() {
            self.delay = new_delay as u64;
        }

        if self.colors.len() > 1 {
            Self::create_ping_pong_gradient(self);
        }
    }

    pub fn do_animation_cycle(&mut self) {
        match self.mode {
            ColorMode::Off => {
                for led in self.controller.leds_mut(0).into_iter() {
                    *led = [0, 0, 0, 0];
                }
                self.controller.render().unwrap();
                self.controller.wait().unwrap();
                thread::sleep(std::time::Duration::from_secs(self.delay / 10));
            }

            ColorMode::Gradient => {
                for j in 0..self.colormap.len() {
                    for (i, led) in self.controller.leds_mut(0).into_iter().enumerate() {
                        let colormap_index = (i + j) % self.colormap.len();
                        *led = self.colormap[colormap_index];
                    }
                    self.controller.render().unwrap();
                    self.controller.wait().unwrap();
                    thread::sleep(std::time::Duration::from_millis(self.delay));
                }
            }

            ColorMode::GradientCenter => {
                for j in 0..self.colormap.len() {
                    let midpoint = LED_COUNT / 2; // Midpoint of the LED strip
                    for i in 0..LED_COUNT {
                        let colormap_index = if i < midpoint {
                            (i + j) % self.colormap.len()
                        } else {
                            (LED_COUNT - 1 - i + j) % self.colormap.len()
                        };
                        self.controller.leds_mut(0)[i] = self.colormap[colormap_index];
                    }
                    self.controller.render().unwrap();
                    self.controller.wait().unwrap();
                    thread::sleep(std::time::Duration::from_millis(self.delay));
                }
            }
            ColorMode::Solid => {
                for led in self.controller.leds_mut(0).into_iter() {
                    *led = self.colors[0];
                }
                self.controller.render().unwrap();
                self.controller.wait().unwrap();
            }
            ColorMode::Twinkle => {
                self.twinkle_driver.do_timestep();
                self.controller.leds_mut(0).copy_from_slice(&self.twinkle_driver.leds());
                self.controller.render().unwrap();
                self.controller.wait().unwrap();
                thread::sleep(std::time::Duration::from_millis(self.delay));
            }
        }
    }
}
