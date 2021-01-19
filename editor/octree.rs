

pub struct Octree<Index, Node> {
    pub nodes: Vec<HashMap<Index, Node>,
    pub depth: usize,
}

impl<Index, Node> Octree<Index, Node> {
    pub fn new(depth: usize) -> Self {
        Self {
            nodes: (0..depth).map(|| HashMap::new()).collect::<Vec<_>>(),
            depth,
        }
    }

    pub fn store(&mut self, lod: usize, index: Index, node: Node) {
        self.nodes[lod].store(&index, node);
    }

    pub fn load(&mut self, lod: usize, index: Index) -> Option<&Node> {
        let node = self.nodes[lod].get(&index);
        if node.is_none() {
            
        }
    }
}
