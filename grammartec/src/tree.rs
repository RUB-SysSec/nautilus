use std::cmp;
use std::io;
use std::io::Error;
use std::io::Write;
use std::marker::Sized;

use context::Context;
use newtypes::{NTermID, NodeID, RuleID};
use rule::{NormalOrCustomRule, Rule, RuleChild};
use std::collections::HashMap;

pub trait TreeLike
where
    Self: Sized,
{
    fn get_rule_id(&self, n: NodeID) -> Option<RuleID>;
    fn size(&self) -> usize;
    fn to_tree(&self, _: &Context) -> Tree;
    fn get_rule<'c, 's: 'c>(&'c self, n: NodeID, ctx: &'s Context) -> &'c Rule;
    fn get_nonterm_id(&self, n: NodeID, ctx: &Context) -> NTermID { self.get_rule(n, ctx).nonterm() } 

    fn unparse<W: Write>(&self, id: NodeID, ctx: &Context, w: &mut W) -> Result<NodeID, Error> {
        return self.get_rule(id, ctx).unparse(self, id, ctx, w);
    }

    fn unparse_iter<W: Write>(&self, id: NodeID, ctx: &Context, w: &mut W) {
        let mut stack: Vec<RuleChild> = Vec::new();
        for i in id.to_i()..self.size() {
            let mut next_nterm = None;
            while let Some(rule_child) = stack.pop() {
                match rule_child {
                    RuleChild::Term(ref data) => {
                        w.write(data).expect("RAND_482559653");
                    }
                    RuleChild::CustomTerm(ref data) => {
                        w.write(data).expect("RAND_3278316750");
                    }
                    RuleChild::NTerm(nterm_id) => {
                        next_nterm = Some(nterm_id);
                        break;
                    }
                }
            }
            let rule = self.get_rule(NodeID::from(i), ctx);
            //sanity check
            if next_nterm.is_some() {
                if next_nterm.expect("RAND_1629372917") != rule.nonterm() {
                    panic!("Not a valid tree for unparsing!");
                }
            }
            for rule_child in rule.children().iter().rev() {
                stack.push(rule_child.clone());
            }
        }
        let mut next_nterm = None;
        while let Some(rule_child) = stack.pop() {
            match rule_child {
                RuleChild::Term(ref data) => {
                    w.write(data).expect("RAND_298992519");
                }
                RuleChild::CustomTerm(ref data) => {
                    w.write(data).expect("RAND_1616900");
                }
                RuleChild::NTerm(nterm_id) => {
                    next_nterm = Some(nterm_id);
                    break;
                }
            }
        }
        if next_nterm.is_some() {
            panic!("Not a valid tree for unparsing!");
        }
    }

    fn unparse_to<W: Write>(&self, ctx: &Context, w: &mut W) -> Result<(), Error> {
        self.unparse_iter(NodeID::from(0), ctx, w);
        return Ok(());
    }

    fn unparse_to_vec(&self, ctx: &Context) -> Vec<u8> {
        self.unparse_node_to_vec(NodeID::from(0), ctx)
    }

    fn unparse_node_to_vec(&self, n: NodeID, ctx: &Context) -> Vec<u8> {
        let mut data = vec![];
        self.unparse_iter(n, ctx, &mut data);
        return data;
    }

    fn unparse_print(&self, ctx: &Context){
        self.unparse_to(ctx, &mut io::stdout());
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tree {
    pub rules: Vec<NormalOrCustomRule>,
    pub sizes: Vec<usize>,
    pub paren: Vec<NodeID>,
}

impl TreeLike for Tree {
    fn get_rule_id(&self, n: NodeID) -> Option<RuleID> {
        match self.rules[n.to_i()] {
            NormalOrCustomRule::NormalRule(rule_id) => {
                return Some(rule_id);
            }
            NormalOrCustomRule::CustomRule(_) => {
                return None;
            }
        }
    }

    fn size(&self) -> usize {
        return self.rules.len();
    }

    fn to_tree(&self, _ctx: &Context) -> Tree {
        return self.clone();
    }

    fn get_rule<'c, 's: 'c>(&'c self, n: NodeID, ctx: &'s Context) -> &'c Rule {
        match self.rules[n.to_i()] {
            NormalOrCustomRule::NormalRule(rule_id) => {
                return ctx.get_rule(rule_id);
            }
            NormalOrCustomRule::CustomRule(ref custom_rule) => {
                return &custom_rule;
            }
        }
    }
}

impl Tree {
    pub fn from_rule_vec(rules: Vec<NormalOrCustomRule>, ctx: &Context) -> Self {
        let sizes = vec![0; rules.len()];
        let paren = vec![NodeID::from(0); rules.len()];
        let mut res = Tree {
            rules,
            sizes,
            paren,
        };
        if res.rules.len() > 0 {
            res.calc_subtree_sizes_and_parents(ctx);
        }
        return res;
    }

    pub fn get_normal_rule_or_custom_rule(&self, n: NodeID) -> &NormalOrCustomRule {
        return &self.rules[n.to_i()];
    }

    pub fn subtree_size(&self, n: NodeID) -> usize {
        return self.sizes[n.to_i()];
    }

    pub fn mutate_replace_from_tree<'a>(
        &'a self,
        n: NodeID,
        other: &'a Tree,
        other_node: NodeID,
    ) -> TreeMutation<'a> {
        let old_size = self.subtree_size(n);
        let new_size = other.subtree_size(other_node);
        return TreeMutation {
            prefix: self.slice(0.into(), n),
            repl: other.slice(other_node, other_node + new_size),
            postfix: self.slice(n + old_size, self.rules.len().into()),
        };
    }

    fn calc_subtree_sizes_and_parents(&mut self, ctx: &Context) {
        self.calc_parents(ctx);
        self.calc_sizes();
    }

    fn calc_parents(&mut self, ctx: &Context) {
        if self.size() == 0 {
            return;
        }
        let mut stack: Vec<(NTermID, NodeID)> = Vec::new();
        stack.push((
            self.get_rule(NodeID::from(0), ctx).nonterm(),
            NodeID::from(0),
        ));
        for i in 0..self.size() {
            let node_id = NodeID::from(i);
            let nonterm = self.get_rule(node_id, ctx).nonterm();
            //sanity check
            let (nterm_id, node) = stack.pop().expect("Not a valid tree for unparsing!");
            if nterm_id != nonterm {
                panic!("Not a valid tree for unparsing!");
            } else {
                self.paren[i] = node;
            }
            let rule = self.get_rule(node_id, ctx);
            for rule_child in rule.children().iter().rev() {
                if let &RuleChild::NTerm(nonterm) = rule_child {
                    stack.push((nonterm, node_id));
                }
            }
        }
    }

    fn calc_sizes(&mut self) {
        //Initiate with 1
        for size in self.sizes.iter_mut() {
            *size = 1;
        }
        for i in (1..self.size()).rev() {
            self.sizes[self.paren[i].to_i()] += self.sizes[i];
        }
    }

    fn slice(&self, from: NodeID, to: NodeID) -> &[NormalOrCustomRule] {
        return &self.rules[from.into()..to.into()];
    }

    pub fn get_parent(&self, n: NodeID) -> Option<NodeID> {
        if n != NodeID::from(0) {
            return Some(self.paren[n.to_i()]);
        }
        return None;
    }

    pub fn truncate(&mut self) {
        self.rules.truncate(0);
        self.sizes.truncate(0);
        self.paren.truncate(0);
    }

    pub fn generate_from_nt(&mut self, start: NTermID, len: usize, ctx: &Context) {
        let ruleid = ctx.get_random_rule_for_nt(start, len);
        self.generate_from_rule(ruleid, len - 1, ctx);
    }

    //Custom Rules all have length 1 and contain only a terminal
    pub fn replace_with_custom_rule(&mut self, node_id: NodeID, new_rule: Rule) {
        let size_difference = self.sizes[node_id.to_i()] - 1;
        let mut current_node_id = node_id;
        while { current_node_id.to_i() != 0 } {
            self.sizes[current_node_id.to_i()] -= size_difference;
            current_node_id = self.paren[current_node_id.to_i()];
        }
        self.sizes[current_node_id.to_i()] -= size_difference;
        //Insert new custom rule
        self.rules[node_id.to_i()] = NormalOrCustomRule::CustomRule(new_rule);
        for x in node_id.to_i() + 1..size_difference - 1 {
            self.rules.remove(x);
            self.paren.remove(x);
            self.sizes.remove(x);
        }
    }

    pub fn generate_from_rule(&mut self, ruleid: RuleID, max_len: usize, ctx: &Context) {
        self.truncate();
        self.rules.push(NormalOrCustomRule::NormalRule(ruleid));
        self.sizes.push(0);
        self.paren.push(NodeID::from(0));
        ctx.get_rule(ruleid).generate(self, &ctx, max_len);
        self.sizes[0] = self.rules.len();
    }

    pub fn has_recursions(&self, ctx: &Context) -> Option<Vec<(NodeID, NodeID)>> {
        let recursions = self.find_recursions_iter(ctx);
        if recursions.len() == 0 {
            return None;
        }
        return Some(recursions);
    }

    fn find_recursions_iter(&self, ctx: &Context) -> Vec<(NodeID, NodeID)> {
        let mut found_recursions = Vec::new();
        //Only search for iterations for up to 10000 nodes
        for i in 1..cmp::min(self.size(), 10000) {
            let node_id = NodeID::from(self.size() - i);
            let mut current_nterm: NTermID = self.get_rule(node_id, ctx).nonterm();
            let mut current_node_id = self.paren[node_id.to_i()];
            let mut depth = 0;
            while current_node_id != NodeID::from(0) {
                if self.get_rule(current_node_id, ctx).nonterm() == current_nterm {
                    found_recursions.push((current_node_id, node_id));
                }
                current_node_id = self.paren[current_node_id.to_i()];
                if depth > 15 {
                    break;
                }
                depth += 1;
            }
        }
        return found_recursions;
    }

}

pub struct TreeMutation<'a> {
    pub prefix: &'a [NormalOrCustomRule],
    pub repl: &'a [NormalOrCustomRule],
    pub postfix: &'a [NormalOrCustomRule],
}

impl<'a> TreeLike for TreeMutation<'a> {
    fn get_rule_id(&self, n: NodeID) -> Option<RuleID> {
        let i = n.to_i();
        let end0 = self.prefix.len();
        let end1 = end0 + self.repl.len();
        let end2 = end1 + self.postfix.len();
        if i < end0 {
            match self.prefix[i] {
                NormalOrCustomRule::NormalRule(rule_id) => {
                    return Some(rule_id);
                }
                NormalOrCustomRule::CustomRule(_) => {
                    return None;
                }
            }
        }
        if i < end1 {
            match self.repl[i - end0] {
                NormalOrCustomRule::NormalRule(rule_id) => {
                    return Some(rule_id);
                }
                NormalOrCustomRule::CustomRule(_) => {
                    return None;
                }
            }
        }
        if i < end2 {
            match self.postfix[i - end1] {
                NormalOrCustomRule::NormalRule(rule_id) => {
                    return Some(rule_id);
                }
                NormalOrCustomRule::CustomRule(_) => {
                    return None;
                }
            }
        }
        panic!("index out of bound for rule access");
    }

    fn size(&self) -> usize {
        return self.prefix.len() + self.repl.len() + self.postfix.len();
    }

    fn to_tree(&self, ctx: &Context) -> Tree {
        let mut vec = vec![];
        vec.extend_from_slice(&self.prefix);
        vec.extend_from_slice(&self.repl);
        vec.extend_from_slice(&self.postfix);
        return Tree::from_rule_vec(vec, ctx);
    }

    fn get_rule<'c, 's: 'c>(&'c self, n: NodeID, ctx: &'s Context) -> &'c Rule {
        let i = n.to_i();
        let end0 = self.prefix.len();
        let end1 = end0 + self.repl.len();
        let end2 = end1 + self.postfix.len();
        if i < end0 {
            match self.prefix[i] {
                NormalOrCustomRule::NormalRule(rule_id) => {
                    return ctx.get_rule(rule_id);
                }
                NormalOrCustomRule::CustomRule(ref custom_rule) => {
                    return &custom_rule;
                }
            }
        }
        if i < end1 {
            match self.repl[i - end0] {
                NormalOrCustomRule::NormalRule(rule_id) => {
                    return ctx.get_rule(rule_id);
                }
                NormalOrCustomRule::CustomRule(ref custom_rule) => {
                    return &custom_rule;
                }
            }
        }
        if i < end2 {
            match self.postfix[i - end1] {
                NormalOrCustomRule::NormalRule(rule_id) => {
                    return ctx.get_rule(rule_id);
                }
                NormalOrCustomRule::CustomRule(ref custom_rule) => {
                    return &custom_rule;
                }
            }
        }
        panic!("index out of bound for rule access");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use context::Context;
    use newtypes::NodeID;
    use std::collections::HashSet;
    use std::iter::FromIterator;

    fn calc_subtree_sizes_and_parents_rec_test(tree: &mut Tree, n: NodeID, ctx: &Context) -> usize {
        let mut cur = n + 1;
        let mut size = 1;
        for _ in 0..tree.get_rule(n, ctx).number_of_nonterms() {
            tree.paren[cur.to_i()] = n;
            let sub_size = calc_subtree_sizes_and_parents_rec_test(tree, cur, ctx);
            cur = cur + sub_size;
            size += sub_size;
        }
        tree.sizes[n.to_i()] = size;
        return size;
    }

    #[test]
    fn check_calc_sizes_iter() {
        let mut ctx = Context::new();
        let _ = ctx.add_rule("C", "c{B}c3");
        let _ = ctx.add_rule("B", "b{A}b23");
        let _ = ctx.add_rule("A", "aasdf {A}");
        let _ = ctx.add_rule("A", "a2 {A}");
        let _ = ctx.add_rule("A", "a sdf{A}");
        let _ = ctx.add_rule("A", "a 34{A}");
        let _ = ctx.add_rule("A", "adfe {A}");
        let _ = ctx.add_rule("A", "a32");
        ctx.initialize(50, false);
        let mut tree = Tree::from_rule_vec(vec![], &ctx);
        for _ in 0..100 {
            tree.truncate();
            tree.generate_from_nt(ctx.nt_id("C"), 50, &ctx);
            calc_subtree_sizes_and_parents_rec_test(&mut tree, NodeID::from(0), &ctx);
            let vec1 = tree.sizes.clone();
            tree.calc_sizes();
            let vec2 = tree.sizes.clone();
            assert_eq!(vec1, vec2);
        }
    }

    #[test]
    fn check_calc_paren_iter() {
        let mut ctx = Context::new();
        let _ = ctx.add_rule("C", "c{B}c3");
        let _ = ctx.add_rule("B", "b{A}b23");
        let _ = ctx.add_rule("A", "aasdf {A}");
        let _ = ctx.add_rule("A", "a2 {A}");
        let _ = ctx.add_rule("A", "a sdf{A}");
        let _ = ctx.add_rule("A", "a 34{A}");
        let _ = ctx.add_rule("A", "adfe {A}");
        let _ = ctx.add_rule("A", "a32");
        ctx.initialize(50, false);
        let mut tree = Tree::from_rule_vec(vec![], &ctx);
        for _ in 0..100 {
            tree.truncate();
            tree.generate_from_nt(ctx.nt_id("C"), 50, &ctx);
            calc_subtree_sizes_and_parents_rec_test(&mut tree, NodeID::from(0), &ctx);
            let vec1 = tree.paren.clone();
            tree.calc_parents(&ctx);
            let vec2 = tree.paren.clone();
            assert_eq!(vec1, vec2);
        }
    }

    #[test]
    fn check_unparse_iter() {
        let mut ctx = Context::new();
        let _ = ctx.add_rule("C", "c{B}c3");
        let _ = ctx.add_rule("B", "b{A}b23");
        let _ = ctx.add_rule("A", "aasdf {A}");
        let _ = ctx.add_rule("A", "a2 {A}");
        let _ = ctx.add_rule("A", "a sdf{A}");
        let _ = ctx.add_rule("A", "a 34{A}");
        let _ = ctx.add_rule("A", "adfe {A}");
        let _ = ctx.add_rule("A", "a32");
        ctx.initialize(50, false);
        let mut tree = Tree::from_rule_vec(vec![], &ctx);
        for _ in 0..100 {
            tree.truncate();
            tree.generate_from_nt(ctx.nt_id("C"), 50, &ctx);
            let mut vec1 = vec![];
            let mut vec2 = vec![];
            tree.unparse(NodeID::from(0), &ctx, &mut vec1)
                .expect("RAND_2991612983");
            tree.unparse_iter(NodeID::from(0), &ctx, &mut vec2);
            assert_eq!(vec1, vec2);
        }
    }

    #[test]
    fn check_find_recursions() {
        let mut ctx = Context::new();
        let _ = ctx.add_rule("C", "c{B}c");
        let _ = ctx.add_rule("B", "b{A}b");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a");
        ctx.initialize(20, false);
        let mut tree = Tree::from_rule_vec(vec![], &ctx);
        for _ in 0..100 {
            tree.truncate();
            tree.generate_from_nt(ctx.nt_id("C"), 20, &ctx);
            let recursions = tree.has_recursions(&ctx).expect("RAND_1192228626");
            assert_ne!(recursions.len(), 0);
            for tuple in recursions {
                assert!(tuple.0.to_i() < tuple.1.to_i());
            }
        }
    }

    #[test]
    fn check_find_recursions_iter() {
        let mut ctx = Context::new();
        let _ = ctx.add_rule("C", "c{B}c");
        let _ = ctx.add_rule("B", "b{A}b");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a");
        ctx.initialize(20, false);
        let mut tree = Tree::from_rule_vec(vec![], &ctx);
        for _ in 0..100 {
            tree.truncate();
            tree.generate_from_nt(ctx.nt_id("C"), 20, &ctx);
            let mut parents = HashMap::new();
            let parent_nonterm = tree.get_rule(NodeID::from(0), &ctx).nonterm();
            parents.insert(parent_nonterm, vec![NodeID::from(0)]);
        }
    }
}
