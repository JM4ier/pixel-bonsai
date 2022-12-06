use rand;
use raylib::prelude::*;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Kind {
    Air,
    Wood,
    Leaf,
    Fruit,
}

impl Default for Kind {
    fn default() -> Self {
        Self::Air
    }
}

#[derive(Copy, Clone, Debug)]
struct Config {
    need_light: u8,
    grow_light: u8,
    grow_probability: f32,
    wood_probability: f32,
    gui_scale: i32,
    leaf_light_succ: u8,
}

#[derive(Clone)]
struct World {
    grid: Vec<Vec<Kind>>,
    light: Vec<Vec<u8>>,
    config: Config,
}

const SIZE: usize = 64;

impl World {
    fn new(config: Config) -> Self {
        let mut new = Self {
            grid: vec![vec![Kind::Air; SIZE]; SIZE],
            light: vec![vec![255; SIZE]; SIZE],
            config,
        };
        new.grid[SIZE / 2][0] = Kind::Wood;
        new.grid[SIZE / 2][1] = Kind::Leaf;
        new
    }
    fn emitting_light(&self, x: usize, y: usize) -> u8 {
        match self.grid[x][y] {
            Kind::Air => self.light[x][y],
            Kind::Leaf if self.light[x][y] >= self.config.leaf_light_succ => {
                self.light[x][y] - self.config.leaf_light_succ
            }
            _ => 0,
        }
    }
    fn update_light(&mut self) {
        for x in 0..SIZE {
            self.light[x][SIZE - 1] = !0;
        }
        for y in (0..SIZE - 1).rev() {
            for x in 0..SIZE {
                let mut light = 0;
                if x > 0 {
                    light = light.max(self.emitting_light(x - 1, y + 1));
                }
                if x + 1 < SIZE {
                    light = light.max(self.emitting_light(x + 1, y + 1));
                }
                light = light.max(self.emitting_light(x, y + 1));
                self.light[x][y] = light;
            }
        }
    }

    fn has_neighbor(&self, x: usize, y: usize, kind: Kind) -> bool {
        let width = 3;
        let fromx = x.max(width) - width;
        let tox = x.min(SIZE - 1 - width) + width;

        if y == 0 {
            return false;
        }

        for nx in fromx..=tox {
            if nx == x {
                continue;
            }
            if self.grid[nx][y - 1] == kind {
                return true;
            }
        }
        false
    }

    fn process(&mut self) {
        for y in (0..SIZE).rev() {
            for x in 0..SIZE {
                match self.grid[x][y] {
                    Kind::Air => {
                        // maybe convert to leaf
                        if self.light[x][y] > self.config.grow_light
                            && self.has_neighbor(x, y, Kind::Leaf)
                            && rand::random::<f32>() < self.config.grow_probability
                        {
                            self.grid[x][y] = Kind::Leaf;
                        }
                    }
                    Kind::Leaf => {
                        // maybe convert to wood
                        if self.light[x][y] < self.config.need_light
                            && self.has_neighbor(x, y, Kind::Wood)
                            && rand::random::<f32>() < self.config.wood_probability
                        {
                            self.grid[x][y] = Kind::Wood;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn simulation_step(&mut self) {
        self.update_light();
        self.process();
    }

    fn render(&self, d: &mut RaylibDrawHandle) {
        for x in 0..SIZE {
            for y in 0..SIZE {
                let color = match self.grid[x][y] {
                    Kind::Air => Color::SKYBLUE,
                    Kind::Leaf => Color::DARKGREEN,
                    Kind::Fruit => Color::RED,
                    Kind::Wood => Color::BROWN,
                };
                let g = self.config.gui_scale;
                let light = Color::new(0, 0, 0, 255 - self.light[x][y]);
                let (x, y) = (x as i32, y as i32);
                let y = SIZE as i32 - y;
                d.draw_rectangle(x * g, y * g, g, g, light);
                d.draw_rectangle(x * g, y * g, g - 2, g - 2, color)
            }
        }
    }
}

pub fn main() {
    let config = Config {
        need_light: 50,
        grow_light: 240,
        grow_probability: 0.001,
        wood_probability: 0.1,
        gui_scale: 10,
        leaf_light_succ: 10,
    };
    let (mut rl, thread) = raylib::init().size(640, 640).title("Marijuana").build();

    let mut world = World::new(config);
    while !rl.window_should_close() {
        world.simulation_step();
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::WHITE);
        world.render(&mut d);
    }
}
