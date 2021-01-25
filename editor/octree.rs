use std::collections::HashMap;

use dotrix::{
    math::Vec3i,
};

const LEFT_TOP_BACK: usize = 0;
const RIGHT_TOP_BACK: usize = 1;
const RIGHT_TOP_FRONT: usize = 2;
const LEFT_TOP_FRONT: usize = 3;
const LEFT_BOTTOM_BACK: usize = 4;
const RIGHT_BOTTOM_BACK: usize = 5;
const RIGHT_BOTTOM_FRONT: usize = 6;
const LEFT_BOTTOM_FRONT: usize = 7;

pub type Key = Vec3i;

pub struct Octree<T> {
    nodes: HashMap<Key, Node<T>>,
    size: usize,
    depth: usize,
}

impl<T> Octree<T> {
    pub fn new(size: usize) -> Self {
        let mut nodes = HashMap::new();

        nodes.insert(Key::new(0, 0, 0), Node {
            level: 0,
            // parent: Key::new(0, 0, 0),
            payload: None,
            children: None,
        });

        Self {
            nodes,
            size,
            depth: 1,
        }
    }

    pub fn store(&mut self, key: Key, payload: T) {
        self.store_child(key, payload, Key::new(0, 0, 0), self.size / 2, 1);
    }

    fn store_child(&mut self, target: Key, payload: T, parent: Key, size: usize, level: usize) {
        // println!("store_child ({:?}, {:?}, {:?}", target, parent, size);
        let offset = size as i32 / 2;

        let node = Key::new(
            (target.x as f32 / size as f32).floor() as i32 * size as i32 + offset,
            (target.y as f32 / size as f32).floor() as i32 * size as i32 + offset,
            (target.z as f32 / size as f32).floor() as i32 * size as i32 + offset
        );
        // let index = Self::child_index((node - parent), offset);

        let payload = {
            let mut child = self.nodes.entry(node.clone()).or_insert(Node {
                level,
                // parent,
                children: None,
                payload: None,
            });

            if node == target {
                child.payload = Some(payload);
                None
            } else {
                if child.children.is_none() {
                    child.children = Some(Node::<T>::children(&node, offset / 2));
                }
                Some(payload)
            }
        };

        if let Some(payload) = payload {
            self.store_child(target, payload, node, offset as usize, level + 1);
        }
    }

    pub fn load(&self, key: &Key) -> Option<&T> {
        self.nodes.get(&key).map(|n| n.payload.as_ref()).unwrap_or(None)
    }

    pub fn find(&self, key: &Key) -> Option<(Key, usize, &T)> {
        if let Some(node) = self.nodes.get(key) {
            if node.payload.is_some() {
                return node.payload.as_ref().map(|p| (*key, node.level, p));
            }
        }
        let half_size = self.size as i32 / 2 + 1;
        if key.x.abs() < half_size && key.y.abs() < half_size && key.z.abs() < half_size {
            self.find_child(key, Key::new(0, 0, 0), self.size / 2)
        } else {
            None
        }
    }

    fn find_child(&self, target: &Key, cursor: Key, size: usize) -> Option<(Key, usize, &T)> {
        let offset = size as i32 / 2;
        let node = Key::new(
            (target.x as f32 / size as f32).floor() as i32 * size as i32 + offset,
            (target.y as f32 / size as f32).floor() as i32 * size as i32 + offset,
            (target.z as f32 / size as f32).floor() as i32 * size as i32 + offset
        );

        if let Some(child) = self.nodes.get(&node) {
            return if child.children.is_some() {
                self.find_child(target, node, offset as usize)
            } else {
                child.payload.as_ref().map(|p| (node, child.level, p))
            };
        }

        // fallback
        self.nodes.get(&cursor)
            .map(|node| node.payload.as_ref().map(|payload| (cursor, node.level, payload)))
            .unwrap_or(None)
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn children(&self, key: &Key) -> Option<&[Vec3i; 8]> {
        self.nodes.get(&key)
            .map(|n| n.children.as_ref())
            .unwrap_or(None)
    }

    /*
    fn child_index(from_root: Vec3i, offset: i32) -> usize {
        if from_root.x == -offset {
            if from_root.z == -offset {
                if from_root.y == offset { return LEFT_TOP_BACK; }
                else if from_root.y == -offset { return LEFT_BOTTOM_BACK; }
            } else if from_root.z == offset {
                if from_root.y == offset { return LEFT_TOP_FRONT; }
                else if from_root.y == -offset { return LEFT_BOTTOM_FRONT; }
            }
        } else if from_root.x == offset {
            if from_root.z == -offset {
                if from_root.y == offset { return RIGHT_TOP_BACK; }
                else if from_root.y == -offset { return RIGHT_BOTTOM_BACK; }
            } else if from_root.z == offset {
                if from_root.y == offset { return RIGHT_TOP_FRONT; }
                else if from_root.y == -offset { return RIGHT_BOTTOM_FRONT; }
            }
        }
        panic!("invalid child offset");
    }
    */

    /*fn parent(&self, key: &Key, level: usize) -> Key {
        let parent_offset = self.size as i32 / (2_i32).pow(level as u32) / 4;

    }*/


}
/*
 *  level   size    offset
    0       32      8
    1       16      4
    2       8       2
    3       4       1
    4       2       -
*/

pub struct Node<T> {
    level: usize,
//    parent: Vec3i,
    children: Option<[Vec3i; 8]>,
    payload: Option<T>,
}

impl<T> Node<T> {
    pub fn children(parent: &Vec3i, offset: i32) -> [Vec3i; 8] {
        let mut res = [Vec3i::new(0, 0, 0); 8];
        for i in 0..8 {
            let (x, y, z) = match i {
                LEFT_TOP_BACK => (parent.x - offset, parent.y + offset, parent.z - offset),
                RIGHT_TOP_BACK => (parent.x + offset, parent.y + offset, parent.z - offset),
                RIGHT_TOP_FRONT => (parent.x + offset, parent.y + offset, parent.z + offset),
                LEFT_TOP_FRONT => (parent.x - offset, parent.y + offset, parent.z + offset),
                LEFT_BOTTOM_BACK => (parent.x - offset, parent.y - offset, parent.z - offset),
                RIGHT_BOTTOM_BACK => (parent.x + offset, parent.y - offset, parent.z - offset),
                RIGHT_BOTTOM_FRONT => (parent.x + offset, parent.y - offset, parent.z + offset),
                LEFT_BOTTOM_FRONT => (parent.x - offset, parent.y - offset, parent.z + offset),
                _ => panic!("cube has only 8 corners"),
            };
            res[i].x = x;
            res[i].y = y;
            res[i].z = z;
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn children_indices_are_correct() {
        let parent = Vec3i::new(0, 0, 0);
        let children = Node::<u32>::children(&parent, 1);
        assert_eq!(children[LEFT_TOP_BACK], Vec3i::new(-1, 1, -1));
        assert_eq!(children[RIGHT_TOP_BACK], Vec3i::new(1, 1, -1));
        assert_eq!(children[RIGHT_TOP_FRONT], Vec3i::new(1, 1, 1));
        assert_eq!(children[LEFT_TOP_FRONT], Vec3i::new(-1, 1, 1));
        assert_eq!(children[LEFT_BOTTOM_BACK], Vec3i::new(-1, -1, -1));
        assert_eq!(children[RIGHT_BOTTOM_BACK], Vec3i::new(1, -1, -1));
        assert_eq!(children[RIGHT_BOTTOM_FRONT], Vec3i::new(1, -1, 1));
        assert_eq!(children[LEFT_BOTTOM_FRONT], Vec3i::new(-1, -1, 1));
    }

    #[test]
    fn can_store_and_load_a_node() {
        let mut octree = Octree::<u32>::new(32);
        octree.store(Key::new(-15, 1, -9), 123);
        assert_eq!(octree.nodes.len(), 5);
        let payload = octree.load(&Key::new(-15, 1, -9));
        assert_eq!(*payload.unwrap(), 123);
    }

    #[test]
    fn can_find_highest_available_lod() {
        let mut octree = Octree::<u32>::new(32);
        octree.store(Key::new(-15, 1, -9), 1);
        octree.store(Key::new(8, 8, 8), 2);

        let (key, &value) = octree.find(&Key::new(-15, 1, -9)).unwrap();
        assert_eq!(key, Key::new(-15, 1, -9));
        assert_eq!(value, 1);

        let (key, &value) = octree.find(&Key::new(4, 0, 4)).unwrap();
        assert_eq!(key, Key::new(8, 8, 8));
        assert_eq!(value, 2);

        let (key, &value) = octree.find(&Key::new(12, 0, 4)).unwrap();
        assert_eq!(key, Key::new(8, 8, 8));
        assert_eq!(value, 2);
    }
}
