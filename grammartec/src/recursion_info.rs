use std::collections::HashMap;
use rand::{sample, thread_rng, Rng, StdRng};

use loaded_dice::LoadedDiceSampler;
use context::Context;
use newtypes::{NodeID, NTermID};
use tree::{Tree,TreeLike};
use rule::NormalOrCustomRule;

struct RecursionInfo {
    recursive_parents: HashMap<NodeID, NodeID>,
    sampler: LoadedDiceSampler<StdRng>,
    depth_by_offset: Vec<usize>,
    node_by_offset: Vec<NodeID>,
}

impl RecursionInfo {

    pub fn new(t: &Tree, n: NTermID, ctx: Context) -> Option<Self> {
        let (recursive_parents, node_by_offset, depth_by_offset)  = RecursionInfo::find_parents(&t, n, &ctx)?;
        let sampler = RecursionInfo::build_sampler(&depth_by_offset);
        return Some(Self{recursive_parents, sampler, node_by_offset, depth_by_offset});
    }

    // constructs a tree where each node points to the first ancestor with the same nonterminal (e.g. each node points the next node above it, were the pair forms a recursive occurance of a nonterminal). 
    // This structure is an ''inverted tree''. We use it later to sample efficiently from the set
    // of all possible recursive pairs without occuring n^2 overhead. Additionally, we return a
    // ordered vec of all nodes with nonterminal n and the depth of this node in the freshly
    // constructed 'recursion tree' (weight). Each node is the end point of exactly `weigth` many
    // differnt recursions. Therefore we use the weight of the node to sample the endpoint of a path trough the
    // recursion tree. Then we just sample the length of this path uniformly as (1.. weight). This
    // yields a uniform sample from the whole set of recursions inside the tree. If you read this, Good luck you are on your own.
    pub fn find_parents(t: &Tree, nt: NTermID, ctx: &Context) -> Option< (HashMap<NodeID, NodeID>, Vec<NodeID>, Vec<usize>) > {
        let mut stack = vec![ (None, 0) ];
        let mut res = None;
        for (i,ref rule_or_custom) in t.rules.iter().enumerate(){
            let node = NodeID::from(i);
            let (mut maybe_parent, depth) = stack.pop().expect("RAND_3404900492");
            if let Some(rule) = rule_or_custom.get_rule_id(){
                if(ctx.get_nt(rule) == nt){
                    if let Some(parent) = maybe_parent {
                        let (mut parents, mut ids, mut weights) = res.unwrap_or_else(|| (HashMap::new(), vec!(), vec!()) );
                        parents.insert(node, parent);
                        ids.push(node);
                        weights.push(depth);
                        res = Some( (parents, ids, weights) );
                    }
                    maybe_parent = Some(node)
                }
                for _ in (0..ctx.get_num_children(rule)){
                    stack.push((maybe_parent, depth+1));
                }
            }
        }
        return res;
    }

    pub fn build_sampler( depths: &Vec<usize> ) -> LoadedDiceSampler<StdRng>{
        let mut weights = depths.iter().map(|x| *x as f64).collect::<Vec<_>>();
        let norm: f64 = weights.iter().sum();
        assert!(norm > 0.0);
        for v in weights.iter_mut(){
            *v /= norm;
        }
        return LoadedDiceSampler::new(weights, StdRng::new().expect("RAND_1769941938"));
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use context::Context;
    use newtypes::NodeID;
    use std::collections::HashSet;
    use std::iter::FromIterator;

    #[test]
    fn check_simple_recursion_info() {
        assert!(false);
    }
}
