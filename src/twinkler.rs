use rand::Rng;

const STAR_SPAWN_RATE: f64 = 0.2;
const COMET_SPAWN_RATE: f64 = 0.01;
const COMET_MIN_TAILSIZE: i32 = 5;
const COMET_MAX_TAILSIZE: i32 = 30;
const COMET_SPEED: u8 = 10;
const DEATH_RATE: f32 = 0.95;

#[derive(Debug)]
struct Comet {
    head_loc: i32,
    tail_size: i32,
    tail_max: i32,
    heat: u8,
    is_dying: bool,
    direction: i32,
    spawning: bool,
    color: [u8; 4],
}

#[derive(Debug)]
struct Star {
    loc: i32,
    heat: u8,
    color: [u8; 4],
    is_dying: bool,
}

#[derive(Debug)]
pub struct Twinkler {
    rng: rand::rngs::ThreadRng,
    stars: Vec<Star>,
    stars_leds: Vec<[u8; 4]>,
    comets: Vec<Comet>,
    comets_leds: Vec<[u8; 4]>,
    led_vec: Vec<[u8; 4]>,
}

impl Twinkler {
    pub fn new() -> Self {
        Twinkler {
            rng: rand::thread_rng(),
            stars: Vec::new(),
            stars_leds: vec![[0, 0, 0, 0]; crate::led_driver::LED_COUNT],
            comets: Vec::new(),
            comets_leds: vec![[0, 0, 0, 0]; crate::led_driver::LED_COUNT],
            led_vec: vec![[0, 0, 0, 0]; crate::led_driver::LED_COUNT],
        }
    }

    pub fn do_timestep(&mut self) {
        if self.rng.gen_bool(STAR_SPAWN_RATE) {
            self.spawn_star();
        }
        if self.rng.gen_bool(COMET_SPAWN_RATE) {
           self.spawn_comet();
        }
        self.clear_leds();
        self.update_stars();
        self.update_comets();
    }

    fn clear_leds(&mut self) {
        self.led_vec = vec![[0, 0, 0, 0]; crate::led_driver::LED_COUNT]
    }

    fn update_stars(&mut self) {
        for star in self.stars.iter_mut() {
            if star.heat > 0 {
                if star.is_dying {
                    star.heat = (star.heat as f32 * DEATH_RATE) as u8;
                } else {
                    star.heat = (((255-star.heat) as f32 * (DEATH_RATE)) as u8).abs_diff(255)
                }
                if (star.heat > 250) {
                    star.is_dying = true;
                }
            }
            self.led_vec[star.loc as usize] = star.color.map(|x| (x as f32 * (star.heat as f32 / 255.0)) as u8);
        }
        self.stars.retain(|x| x.heat != 0);
    }

    fn update_comets(&mut self) {
        for comet in self.comets.iter_mut() {
            comet.head_loc += comet.direction;
            if comet.heat > 0 {
                if comet.is_dying {
                    comet.heat = (comet.heat as f32 * DEATH_RATE) as u8;
                } else {
                    comet.heat = (((255-comet.heat) as f32 * (DEATH_RATE)) as u8).abs_diff(255)
                }
                if (comet.heat > 250) {
                    comet.is_dying = true;
                }
            }
            for i in 0..comet.tail_max {
                let m = (comet.head_loc + i * comet.direction * -1)
                    .clamp(0, crate::led_driver::LED_COUNT as i32 -1) as usize;
                let l = comet.color.map(|x| {
                    (x as f32 * (comet.heat as f32 * (1.0 - i as f32/comet.tail_max as f32)) / 255.0)
                        .clamp(0.0, 255.0) as u8});
                if l.iter().any(|x| *x != 0) {
                    self.led_vec[m] = l;
                }
                
            }
        }
        self.comets.retain(|x| x.heat != 0);
    }

    fn spawn_star(&mut self) {
        self.stars.push(Star {
            loc: self.rng.gen_range(0..crate::led_driver::LED_COUNT as i32),
            heat: 1,
            color: [255u8, 100, 0, 0],
            is_dying: false
        });
    }

    fn spawn_comet(&mut self) {
        let loc = self.rng.gen_range(0..crate::led_driver::LED_COUNT as i32);
        self.comets.push(Comet {
            head_loc: loc,
            tail_size: 0,
            tail_max: self.rng.gen_range(COMET_MIN_TAILSIZE..COMET_MAX_TAILSIZE),
            heat: 1,
            is_dying: false,
            direction: if self.rng.gen_bool(0.5) { 1 } else { -1 },
            spawning: true,
            color: [0u8, 0, 255, 0],
        });
    }

    pub fn leds(&self) -> Vec<[u8; 4]> {
        self.led_vec.clone()
    }
}
