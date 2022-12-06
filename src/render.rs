use crate::*;

pub type LightMap = Vec<Vec<Vec<f32>>>;

pub(crate) struct PrettyRender {
    /// the tree we render
    tree: Tree,
    /// Amount of shade
    ///
    /// 0 = no shade,
    /// 1 = full shade
    shade: LightMap,
}

fn lightmap(tree: &Tree) -> LightMap {
    let width = tree.config.width as usize / tree.config.pixel_size;
    let height = tree.config.height as usize / tree.config.pixel_size;
    let depth = tree.config.node_depth_max;
    let mut map = vec![vec![vec![0.0; depth]; height]; width];
    let mut draw_sphere = |global: bool, center: Vector3, radius: f32, alpha: f32| {
        todo!("convert vector to local coords??");

        let r = radius;
        let begin = |x| if x - r < 0.0 { 0 } else { (x - r) as usize };
        let end = |x, size: usize| {
            if x + r >= size as f32 {
                size - 1
            } else {
                (x + r) as usize
            }
        };
        let (bx, by) = (begin(center.x), begin(center.y));
        let (ex, ey) = (end(center.x, width), end(center.y, height));

        let (bz, ez) = if global {
            (0, tree.config.node_depth_max)
        } else {
            (
                center.z as usize,
                (1 + center.z as usize).min(tree.config.node_depth_max - 1),
            )
        };

        for x in bx..=ex {
            for y in by..=ey {
                for z in bz..=ez {
                    let pos = Vector3::new(x as _, y as _, z as _);
                    if (pos - center).length() < radius {
                        map[x][y][z] = alpha;
                    }
                }
            }
        }
    };
    for node in tree.nodes.iter() {
        let pos = Vector3::new(node.pos.x, node.pos.y, node.z);
        let parent_pos = if let Some(parent_idx) = node.parent {
            let p = tree.nodes[parent_idx];
            Vector3::new(p.pos.x, p.pos.y, p.z)
        } else {
            pos - Vector3::new(0.0, tree.config.grow_dist, 0.0)
        };
        for i in 0..10 {
            let interp_pos = pos.lerp(parent_pos, i as f32 * 0.1);
            draw_sphere(false, interp_pos, tree.radius_of(node), 1.0);
        }
    }
    for node in tree.nodes.iter() {
        if tree.radius_of(node) >= tree.config.leaf_max_width {
            continue;
        }
        let pos = Vector3::new(node.pos.x, node.pos.y, node.z);
        draw_sphere(true, pos, tree.config.leaf_size, 0.1);
    }
    for x in 1..map.len() {
        for y in (0..map[x].len() - 1).rev() {
            for z in 0..map[x][y].len() {
                map[x][y][z] = map[x][y][z].max(map[x - 1][y + 1][z]);
            }
        }
    }
    map
}

impl PrettyRender {
    /// Creates a new renderer
    /// Expensive shading computations
    pub fn new(tree: Tree) -> Self {
        let shade = lightmap(&tree);
        Self { tree, shade }
    }
}

impl PrettyRender {
    pub fn render(&self, d: &mut RaylibDrawHandle) {}
}
