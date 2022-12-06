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
    let mut draw_sphere = |center: Vector3, radius: f32, alpha: f32| {
        let r = radius;
        let begin = |x| if x - r < 0.0 { 0 } else { (x - r) as usize };
        let end = |x, size: usize| {
            if x + r >= size as f32 {
                size - 1
            } else {
                (x + r) as usize
            }
        };
        let (bx, by, bz) = (begin(center.x), begin(center.y), begin(center.z));
        let (ex, ey, ez) = (
            end(center.x, width),
            end(center.y, height),
            end(center.z, depth),
        );

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
            draw_sphere(interp_pos, tree.radius_of(node), 1.0);
        }
    }
    todo!();
    map
}

impl PrettyRender {
    /// Creates a new renderer
    /// Expensive shading computations
    pub fn new(tree: Tree) -> Self {
        let config = tree.config;

        let shade = lightmap(&tree);

        Self { tree, shade }
    }
}

impl PrettyRender {
    pub fn render(&self, d: &mut RaylibDrawHandle) {}
}
