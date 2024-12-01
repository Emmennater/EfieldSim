use crate::body::Body;
use ultraviolet::Vec2;

#[derive(Clone, Copy)]
pub struct Quad {
    pub center: Vec2,
    pub size: f32,
}

impl Quad {
    pub fn new_containing(bodies: &[Body]) -> Self {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for body in bodies {
            min_x = min_x.min(body.pos.x);
            min_y = min_y.min(body.pos.y);
            max_x = max_x.max(body.pos.x);
            max_y = max_y.max(body.pos.y);
        }

        let center = Vec2::new(min_x + max_x, min_y + max_y) * 0.5;
        let size = (max_x - min_x).max(max_y - min_y);

        Self { center, size }
    }

    pub fn find_quadrant(&self, pos: Vec2) -> usize {
        ((pos.y > self.center.y) as usize) << 1 | (pos.x > self.center.x) as usize
    }

    pub fn into_quadrant(mut self, quadrant: usize) -> Self {
        self.size *= 0.5;
        self.center.x += ((quadrant & 1) as f32 - 0.5) * self.size;
        self.center.y += ((quadrant >> 1) as f32 - 0.5) * self.size;
        self
    }

    pub fn subdivide(&self) -> [Quad; 4] {
        [0, 1, 2, 3].map(|i| self.into_quadrant(i))
    }
}

#[derive(Clone)]
pub struct Node {
    pub children: usize,
    pub next: usize,
    pub pos: Vec2,
    pub charge: f32,
    pub quad: Quad,
}

impl Node {
    pub fn new(next: usize, quad: Quad) -> Self {
        Self {
            children: 0,
            next,
            pos: Vec2::zero(),
            charge: 0.0,
            quad,
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.children == 0
    }

    pub fn is_branch(&self) -> bool {
        self.children != 0
    }

    pub fn is_empty(&self) -> bool {
        self.charge == 0.0
    }
}

pub struct Quadtree {
    pub t_sq: f32,
    pub e_sq: f32,
    pub nodes: Vec<Node>,
    pub parents: Vec<usize>,
    pub calcs: usize,
}

impl Quadtree {
    pub const ROOT: usize = 0;

    pub fn new(theta: f32, epsilon: f32) -> Self {
        Self {
            t_sq: theta * theta,
            e_sq: epsilon * epsilon,
            nodes: Vec::new(),
            parents: Vec::new(),
            calcs: 0,
        }
    }

    pub fn clear(&mut self, quad: Quad) {
        // println!("Calculations: {0}", self.calcs);
        
        self.nodes.clear();
        self.parents.clear();
        self.nodes.push(Node::new(0, quad));
        self.calcs = 0;
    }

    fn subdivide(&mut self, node: usize) -> usize {
        self.parents.push(node);
        let children = self.nodes.len();
        self.nodes[node].children = children;

        let nexts = [
            children + 1,
            children + 2,
            children + 3,
            self.nodes[node].next,
        ];
        let quads = self.nodes[node].quad.subdivide();
        for i in 0..4 {
            self.nodes.push(Node::new(nexts[i], quads[i]));
        }

        return children;
    }

    pub fn insert(&mut self, pos: Vec2, charge: f32) {
        let mut node = Self::ROOT;

        while self.nodes[node].is_branch() {
            let quadrant = self.nodes[node].quad.find_quadrant(pos);
            node = self.nodes[node].children + quadrant;
        }

        if self.nodes[node].is_empty() {
            self.nodes[node].pos = pos;
            self.nodes[node].charge = charge;
            return;
        }

        let (p, m) = (self.nodes[node].pos, self.nodes[node].charge);
        if pos == p {
            self.nodes[node].charge += charge;
            return;
        }

        loop {
            let children = self.subdivide(node);

            let q1 = self.nodes[node].quad.find_quadrant(p);
            let q2 = self.nodes[node].quad.find_quadrant(pos);

            if q1 == q2 {
                node = children + q1;
            } else {
                let n1 = children + q1;
                let n2 = children + q2;

                self.nodes[n1].pos = p;
                self.nodes[n1].charge = m;
                self.nodes[n2].pos = pos;
                self.nodes[n2].charge = charge;
                return;
            }
        }
    }

    pub fn propagate(&mut self) {
        for &node in self.parents.iter().rev() {
            let i = self.nodes[node].children;

            self.nodes[node].pos = self.nodes[i].pos * self.nodes[i].charge
                + self.nodes[i + 1].pos * self.nodes[i + 1].charge
                + self.nodes[i + 2].pos * self.nodes[i + 2].charge
                + self.nodes[i + 3].pos * self.nodes[i + 3].charge;

            self.nodes[node].charge = self.nodes[i].charge
                + self.nodes[i + 1].charge
                + self.nodes[i + 2].charge
                + self.nodes[i + 3].charge;

            let mass = self.nodes[node].charge;
            self.nodes[node].pos /= mass;
        }
    }

    pub fn efield(&mut self, pos: Vec2) -> Vec2 {
        let mut efield = Vec2::zero();

        let mut node = Self::ROOT;
        loop {
            let n = &self.nodes[node];

            let d = pos - n.pos;
            let d_sq = d.mag_sq();

            if n.is_leaf() || n.quad.size * n.quad.size < d_sq * self.t_sq {
                // Electic force (2D)
                let denom = d_sq + self.e_sq;
                efield += d * (n.charge / denom).min(f32::MAX);
                self.calcs += 1;

                if n.next == 0 {
                    break;
                }
                node = n.next;
            } else {
                node = n.children;
            }
        }

        efield
    }
}
