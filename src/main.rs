#![feature(drain_filter)]

mod render;

use std::ops::Add;

use fuss::Simplex;
use rand::{rngs::ThreadRng, Rng};
use raylib::prelude::*;

struct SimplexDensityPRG {
    buf: Vec<Vec<f32>>,
    rows: Vec<f32>,
    sum: f32,
}

impl SimplexDensityPRG {
    pub fn new(width: usize, height: usize) -> Self {
        let noise = Simplex::new();
        let mut buf = vec![vec![0f32; height]; width];
        let mut rows = vec![0f32; width];
        let mut sum = 0f32;
        for x in 0..width {
            for y in 0..height {
                let noise_val = noise.sum_octave_2d(3, x as _, y as _, 0.5, 0.003).abs();
                let centering = {
                    let width = width as f32;
                    let height = height as f32;

                    let midx = width / 2.0;
                    let dx = x as f32 - midx;

                    let midy = height / 2.0;
                    let dy = y as f32 - midy;

                    let edge_pow = 3.5;

                    let dist = dx.abs().powf(edge_pow) + dy.abs().powf(edge_pow);
                    let fade0 = (0.5 * width.min(height)).powf(edge_pow);
                    let fade1 = (0.8f32).powf(edge_pow) * fade0;

                    let v = (dist - fade0) / (fade1 - fade0);
                    let v = v.max(0.0).min(1.0);

                    // sin curve mapping 0,1 to 0,1 to have a smooth gradient
                    (((v - 0.5) * std::f32::consts::PI).sin() + 1.0) * 0.5
                };
                let buf_val = noise_val * centering;
                buf[x][y] = buf_val;
                rows[x] += buf_val;
                sum += buf_val;
            }
        }
        Self { buf, rows, sum }
    }
    pub fn sample(&self, rand: &mut ThreadRng) -> (usize, usize) {
        let rand = rand.gen::<f32>();
        assert!(0.0 <= rand && rand < 1.0);
        let mut rand = rand * self.sum;

        let mut x = 0;
        while x < self.buf.len() && rand >= self.rows[x] {
            rand -= self.rows[x];
            x += 1;
        }

        if x == self.buf.len() {
            x -= 1;
        }

        let mut y = 0;
        while y < self.buf[x].len() && rand >= self.buf[x][y] {
            rand -= self.buf[x][y];
            y += 1;
        }

        (x, y)
    }
}

#[derive(Debug, Copy, Clone)]
struct ColorPalette {
    leaf: Color,
    new_branch: Color,
    old_branch: Color,
}

#[derive(Debug, Copy, Clone)]
struct Config {
    attraction_dist: f32,
    kill_dist: f32,
    grow_dist: f32,
    node_min_dist: f32,
    width: f32,
    height: f32,
    max_children: usize,
    max_depth: usize,
    num_points: usize,
    min_y_growth: f32,
    parent_dir_factor: f32,
    weight_display_pow: f32,
    prune_pow: f32,
    prune_size_ratio: f32,
    /// Maximum branch width to grow leaves there
    leaf_max_width: f32,
    /// Maximum branch width to color the branch green
    sprout_max_width: f32,
    leaf_size: f32,
    colors: ColorPalette,
    node_depth_change: f32,
    node_depth_max: usize,
    /// How big one "pixel" is (in pixels)
    pixel_size: usize,
}

#[derive(Debug, Copy, Clone)]
struct Node {
    alive: bool,
    pos: Vector2,
    parent: Option<usize>,
    child_count: usize,
    /// distance to root
    depth: usize,
    /// amount of children attached to this node + 1
    weight: usize,
    z: f32,
}

impl Node {
    fn new_root(pos: Vector2) -> Self {
        Self {
            alive: true,
            pos,
            parent: None,
            child_count: 0,
            depth: 0,
            weight: 1,
            z: 0.0,
        }
    }
    fn new_branch(pos: Vector2, parent_idx: usize, parent: Node, config: Config) -> Self {
        let z_change = (2.0 * rand::random::<f32>() - 1.0) * config.node_depth_change;
        let z = parent.z + z_change;
        let z = z.max(0.0).min(config.node_depth_max as _);
        Self {
            alive: true,
            pos,
            parent: Some(parent_idx),
            child_count: 0,
            depth: parent.depth + 1,
            weight: 1,
            z,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum DrawMode {
    Debug,
    Pretty,
}

struct Tree {
    nodes: Vec<Node>,
    config: Config,
    points: Vec<Vector2>,
    growing: bool,
}

impl Tree {
    fn new_min_growth(config: Config, iter: usize) -> Self {
        let mut tree = Self::new(config);
        for _ in 0..iter {
            tree.sim();
        }
        if tree.nodes.len() < iter {
            Self::new_min_growth(config, iter)
        } else {
            tree
        }
    }
    fn new(config: Config) -> Self {
        let prg_map = SimplexDensityPRG::new(config.width as _, config.height as _);
        let points = (0..config.num_points)
            .map(|_| {
                let (x, y) = prg_map.sample(&mut rand::thread_rng());
                Vector2::new(x as f32, y as f32)
            })
            .collect::<Vec<_>>();
        Self {
            nodes: vec![Node::new_root(Vector2::new(
                config.width / 2.0,
                config.height / 10.0,
            ))],
            config,
            points,
            growing: true,
        }
    }
    fn render(&self, d: &mut RaylibDrawHandle, mode: DrawMode) {
        let map_pos = |pos: &Vector2| Vector2::new(pos.x, self.config.height - pos.y);
        match mode {
            DrawMode::Debug => {
                for point in &self.points {
                    d.draw_circle_v(map_pos(point), 0.99, Color::BLACK);
                }
                for node in self.nodes.iter() {
                    let color = if node.alive { Color::BLUE } else { Color::RED };
                    let pos = map_pos(&node.pos);
                    if let Some(parent_idx) = node.parent {
                        d.draw_line_v(map_pos(&self.nodes[parent_idx].pos), pos, color);
                    }
                    d.draw_circle_v(pos, self.radius_of(node), color);
                }
            }
            DrawMode::Pretty => {
                for node in self.nodes.iter().filter(|n| n.alive) {
                    let mut leaf = false;
                    let radius = self.radius_of(node);

                    let color = if radius < self.config.leaf_max_width {
                        leaf = true;
                        self.config.colors.leaf
                    } else if radius < self.config.sprout_max_width {
                        self.config.colors.new_branch
                    } else {
                        self.config.colors.old_branch
                    };

                    let pos = map_pos(&node.pos);
                    if let Some(parent_idx) = node.parent {
                        for i in 0..10 {
                            let f = (i as f32 + 1.0) / 10.0;
                            d.draw_circle_v(
                                pos.lerp(map_pos(&self.nodes[parent_idx].pos), f),
                                radius,
                                color,
                            );
                        }
                    }
                    d.draw_circle_v(pos, radius, color);
                    if leaf {
                        d.draw_circle_v(pos, self.config.leaf_size, color.fade(0.1));
                    }
                }
            }
        }
    }

    pub fn radius_of(&self, node: &Node) -> f32 {
        0.5 + (node.weight as f32).powf(self.config.weight_display_pow)
    }

    fn sim(&mut self) {
        if !self.growing {
            return;
        }
        let mut new_nodes = vec![];
        for (node_idx, node) in self.nodes.iter().enumerate() {
            if node.child_count >= self.config.max_children || !node.alive {
                continue;
            }
            let near_points = self
                .points
                .iter()
                .map(|p| *p - node.pos)
                .filter(|p| {
                    p.length_sqr() < self.config.attraction_dist * self.config.attraction_dist
                })
                .collect::<Vec<_>>();
            if near_points.is_empty() {
                continue;
            }
            let avg_dir = near_points
                .into_iter()
                .fold(Vector2::zero(), Add::add)
                .normalized()
                * self.config.grow_dist;

            // in similar dir as parent
            let prev_dir = if let Some(parent) = node.parent {
                node.pos - self.nodes[parent].pos
            } else {
                Vector2::new(0.0, self.config.grow_dist)
            };
            let delta = avg_dir.lerp(prev_dir, self.config.parent_dir_factor);

            new_nodes.push(Node::new_branch(
                node.pos + delta,
                node_idx,
                *node,
                self.config,
            ));
        }
        self.points
            .drain_filter(|p| {
                self.nodes.iter().any(|node| {
                    (*p - node.pos).length_sqr() < self.config.kill_dist * self.config.kill_dist
                })
            })
            .last();
        let mut has_change = false;
        'outer: for node in new_nodes.into_iter() {
            if node.depth > self.config.max_depth
                || node.pos.y - self.nodes[node.parent.unwrap()].pos.y < self.config.min_y_growth
            {
                continue 'outer;
            }
            for nod in self.nodes.iter() {
                if (nod.pos - node.pos).length_sqr()
                    < self.config.node_min_dist * self.config.node_min_dist
                {
                    continue 'outer;
                }
            }
            self.nodes[node.parent.unwrap()].child_count += 1;
            self.nodes.push(node);
            has_change = true;
        }
        self.growing &= has_change;

        self.prune();
        self.recalculate_weight();
    }
    /// Kills small branches that are too close to big branches
    fn prune(&mut self) {
        let mut death_node = vec![];
        for (node_idx, node) in self.nodes.iter().enumerate() {
            for conflict in self.nodes.iter() {
                let distance = (conflict.pos - node.pos).length();
                if (node.weight as f32) < self.config.prune_size_ratio * conflict.weight as f32
                    && distance < (conflict.weight as f32).powf(self.config.prune_pow)
                {
                    death_node.push(node_idx);
                }
            }

            // transitive adding of dead nodes
            let mut ancestor = node.parent;
            while let Some(ancestor_idx) = ancestor {
                if death_node.contains(&ancestor_idx) {
                    death_node.push(node_idx);
                }
                ancestor = self.nodes[ancestor_idx].parent;
            }
        }

        for death in death_node {
            self.nodes[death].alive = false;
        }
    }
    fn recalculate_weight(&mut self) {
        for node in self.nodes.iter_mut() {
            node.weight = 1;
        }

        for node_idx in (0..self.nodes.len()).rev() {
            let node = self.nodes[node_idx].clone();
            if let Some(parent_idx) = node.parent {
                self.nodes[parent_idx].weight += node.weight;
            }
        }
    }
}

pub fn main() {
    let colors = ColorPalette {
        leaf: Color::GREEN,
        new_branch: Color::GREEN,
        old_branch: Color::BROWN,
    };
    let config = Config {
        attraction_dist: 20.0,
        kill_dist: 13.0,
        grow_dist: 10.0,
        node_min_dist: 8.0,
        width: 500.0,
        height: 500.0,
        max_children: 3,
        max_depth: 5000,
        num_points: 10_000,
        min_y_growth: 0.0,
        parent_dir_factor: 0.1,
        weight_display_pow: 0.35,
        prune_pow: 0.35,
        prune_size_ratio: 0.2,
        leaf_max_width: 3.0,
        sprout_max_width: 3.5,
        leaf_size: 25.0,
        node_depth_change: 1.0,
        node_depth_max: 5,
        pixel_size: 5,
        colors,
    };

    let (mut rl, thread) = raylib::init()
        .size(config.width as _, config.height as _)
        .title("hehe")
        .build();

    let mut regenerated = false;
    'regenerate: while !rl.window_should_close() {
        let mut tree = Tree::new_min_growth(config, 5);

        rl.set_target_fps(60);

        while !rl.window_should_close() {
            if rl.is_key_down(KeyboardKey::KEY_R) && !regenerated {
                regenerated = true;
                continue 'regenerate;
            }
            let mut d = rl.begin_drawing(&thread);
            d.clear_background(Color::WHITE);
            tree.render(&mut d, DrawMode::Pretty);
            tree.sim();
            regenerated = false;
        }

        break;
    }
}
