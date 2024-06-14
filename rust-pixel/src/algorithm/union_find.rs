// RustPixel
// copyright zhouxin@tuyoogame.com 2022~2024


//! disjoint-set data structure, reference:
//! https://chiclaim.blog.csdn.net/article/details/80721436

/// UF interface
pub trait UF {
    fn is_connect(&mut self, p: usize, q: usize) -> bool;
    fn union(&mut self, p: usize, q: usize);
    fn get_size(&self) -> usize;
}

/// UnionFind struct
pub struct UnionFind {
    // parent node
    parent: Vec<usize>,
    // height
    rank: Vec<usize>,
}

impl UnionFind {
    pub fn new(size: usize) -> Self {
        let mut res = Self {
            parent: vec![0_usize; size],
            rank: vec![1_usize; size],
        };
        for i in 0..size {
            res.parent[i] = i;
        }
        res
    }

    pub fn find(&mut self, p: usize) -> Result<usize, &'static str> {
        if p >= self.parent.len() {
            return Err("paramter error");
        }
        let mut c = p;
        while c != self.parent[c] {
            // compress height
            self.parent[c] = self.parent[self.parent[c]];
            c = self.parent[c];
        }
        return Ok(c);
    }
}

impl UF for UnionFind {
    fn is_connect(&mut self, p: usize, q: usize) -> bool {
        let p_root = self.find(p).unwrap();
        let q_root = self.find(q).unwrap();
        return p_root == q_root;
    }

    fn union(&mut self, p: usize, q: usize) {
        let p_root = self.find(p).unwrap();
        let q_root = self.find(q).unwrap();
        //re-balancing based on height
        if p_root != q_root {
            if self.rank[p_root] < self.rank[q_root] {
                self.parent[p_root] = self.parent[q_root];
            } else if self.rank[q_root] < self.rank[p_root] {
                self.parent[q_root] = self.parent[p_root];
            } else {
                self.parent[q_root] = self.parent[p_root];
                self.rank[p_root] += 1;
            }
        }
    }

    fn get_size(&self) -> usize {
        return self.parent.len();
    }
}

