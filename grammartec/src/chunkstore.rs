use std::collections::HashMap;
// use std::collections::HashSet;
use rand::{sample, thread_rng};
use std::sync::atomic::AtomicBool;
use std::sync::RwLock;

use context::Context;
use newtypes::{NTermID, NodeID, RuleID};
use tree::{Tree, TreeLike};

pub struct ChunkStoreWrapper {
    pub chunkstore: RwLock<ChunkStore>,
    pub is_locked: AtomicBool,
}
impl ChunkStoreWrapper {
    pub fn new() -> Self {
        return ChunkStoreWrapper {
            chunkstore: RwLock::new(ChunkStore::new()),
            is_locked: AtomicBool::new(false),
        };
    }
}

#[derive(Serialize, Deserialize)]
pub struct ChunkStore {
    nts_to_chunks: HashMap<NTermID, Vec<(usize, NodeID)>>,
    //seen_outputs: HashSet<Vec<u8>>,
    trees: Vec<Tree>,
}

impl ChunkStore {
    pub fn new() -> Self {
        return ChunkStore {
            nts_to_chunks: HashMap::new(),
            /*seen_outputs: HashSet::new(),*/ trees: vec![],
        };
    }

    pub fn add_tree(&mut self, tree: Tree, ctx: &Context) {
        //let mut buffer = vec!();
        let id = self.trees.len();
        for i in 0..tree.size() {
            //buffer.truncate(0);
            if tree.sizes[i] > 30 {
                continue;
            }
            let n = NodeID::from(i);
            //tree.unparse_iter(n,ctx, &mut buffer);
            //if !self.seen_outputs.contains(&buffer) {
            //    self.seen_outputs.insert(buffer.clone());
            self.nts_to_chunks
                .entry(tree.get_rule(n, ctx).nonterm())
                .or_insert_with(|| vec![])
                .push((id, n));
            //}
        }
        self.trees.push(tree);
    }

    pub fn get_alternative_to<'a>(&'a self, r: RuleID, ctx: &Context) -> Option<(&Tree, NodeID)> {
        let chunks = self.nts_to_chunks.get(&ctx.get_nt(r));
        let relevant = chunks.map(|vec| {
            vec.iter()
                .filter(move |&&(tid, nid)| self.trees[tid].get_rule_id(nid) != Some(r))
        });
        let selected = relevant.and_then(|iter| sample(&mut thread_rng(), iter, 1).pop());
        return selected.map(|&(tid, nid)| (&self.trees[tid], nid));
    }

    pub fn trees(&self) -> usize {
        return self.trees.len();
    }

}

#[cfg(test)]
mod tests {
    use chunkstore::ChunkStore;
    use context::Context;
    use tree::TreeLike;

    #[test]
    fn chunk_store() {
        let mut ctx = Context::new();
        let r1 = ctx.add_rule("A", "a {B:a}");
        let r2 = ctx.add_rule("B", "b {C:a}");
        let _ = ctx.add_rule("C", "c");
        ctx.initialize(101, false);
        let random_size = ctx.get_random_len_for_ruleid(&r1);
        println!("random_size: {}", random_size);
        let tree = ctx.generate_tree_from_rule(r1, random_size);
        let mut cks = ChunkStore::new();
        cks.add_tree(tree, &ctx);
        // assert!(cks.seen_outputs.contains("a b c".as_bytes()));
        // assert!(cks.seen_outputs.contains("b c".as_bytes()));
        // assert!(cks.seen_outputs.contains("c".as_bytes()));
        assert_eq!(cks.nts_to_chunks[&ctx.nt_id("A")].len(), 1);
        let (tree_id, _) = cks.nts_to_chunks[&ctx.nt_id("A")][0];
        assert_eq!(cks.trees[tree_id].unparse_to_vec(&ctx), "a b c".as_bytes());

        let random_size = ctx.get_random_len_for_ruleid(&r2);
        let tree = ctx.generate_tree_from_rule(r2, random_size);
        cks.add_tree(tree, &ctx);
        // assert_eq!(cks.seen_outputs.len(), 3);
        // assert_eq!(cks.nts_to_chunks[&ctx.nt_id("B")].len(), 1);
        let (tree_id, node_id) = cks.nts_to_chunks[&ctx.nt_id("B")][0];
        assert_eq!(
            cks.trees[tree_id].unparse_node_to_vec(node_id, &ctx),
            "b c".as_bytes()
        );
    }
}
