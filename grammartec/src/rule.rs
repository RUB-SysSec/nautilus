//use std::collections::HashMap;
use num::Zero;
use std::io::Write;

use context::Context;
use newtypes::{NTermID, NodeID, RuleID};
use regex::Regex;
use std::io::Error;
use tree::{Tree, TreeLike};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum RuleChild {
    Term(Vec<u8>),
    CustomTerm(Vec<u8>),
    NTerm(NTermID),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum NormalOrCustomRule {
    NormalRule(RuleID),
    CustomRule(Rule),
}

impl NormalOrCustomRule {
    pub fn get_rule_id(&self) -> Option<RuleID>{
        if let NormalOrCustomRule::NormalRule(r) = self {return Some(*r)}
        return None
    }
}

impl RuleChild {
    pub fn from_lit(lit: &str) -> Self {
        return RuleChild::Term(lit.into());
    }

    pub fn from_nt(nt: &str, ctx: &mut Context) -> Self {
        let (nonterm, _) = RuleChild::split_nt_description(nt);
        return RuleChild::NTerm(ctx.aquire_nt_id(&nonterm));
    }

    pub fn unparse<W: Write, T: TreeLike>(
        &self,
        tree: &T,
        mut cur: NodeID,
        ctx: &Context,
        w: &mut W,
    ) -> Result<NodeID, Error> {
        match self {
            &RuleChild::Term(ref data) => {
                w.write(data)?;
            }
            &RuleChild::CustomTerm(ref data) => {
                w.write(data)?;
            }
            &RuleChild::NTerm(_) => {
                cur = tree.unparse(cur + 1, ctx, w)?;
            }
        }
        return Ok(cur);
    }

    fn split_nt_description(nonterm: &str) -> (String, String) {
        lazy_static! {
            static ref SPLITTER: Regex = Regex::new(
                r"^\{([A-Z][a-zA-Z_\-0-9]*)(?::([a-zA-Z_\-0-9]*))?\}$"
            ).expect("RAND_1363289094");
        }

        //splits {A:a} or {A} into A and maybe a
        let descr = SPLITTER.captures(nonterm).expect("RAND_3427632992");
        //let name = descr.get(2).map(|m| m.as_str().into()).unwrap_or(default.to_string()));
        return (descr[1].into(), "".into());
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Rule {
    nonterm: NTermID,
    children: Vec<RuleChild>,
    nonterms: Vec<NTermID>,
}

impl Rule {
    pub fn from_format(ctx: &mut Context, nonterm: &str, format: &str) -> Self {
        let children = Rule::tokenize(format, ctx);
        let nonterms = children
            .iter()
            .filter_map(|c| {
                if let &RuleChild::NTerm(n) = c {
                    Some(n)
                } else {
                    None
                }
            })
            .collect();
        return Rule {
            nonterm: ctx.aquire_nt_id(nonterm),
            children,
            nonterms,
        };
    }

    pub fn from_term(ntermid: NTermID, term: &Vec<u8>) -> Self {
        let children = vec![RuleChild::Term(term.to_vec())];
        let nonterms = vec![];
        return Rule {
            nonterm: ntermid,
            children,
            nonterms,
        };
    }

    pub fn from_custom_term(ntermid: NTermID, term: Vec<u8>) -> Self {
        let children = vec![RuleChild::CustomTerm(term)];
        let nonterms = vec![];
        return Rule {
            nonterm: ntermid,
            children,
            nonterms,
        };
    }

    fn tokenize(format: &str, ctx: &mut Context) -> Vec<RuleChild> {
        lazy_static! {
            static ref TOKENIZER: Regex =
                Regex::new(r"(\{[^}\\]+\})|((?:[^{\\]|\\\{|\\\}|\\)+)").expect("RAND_994455541");
        } //RegExp Changed from (\{[^}\\]+\})|((?:[^{\\]|\\\{|\\\}|\\\\)+) because of problems with \\ (\\ was not matched and therefore thrown away)

        return TOKENIZER
            .captures_iter(format)
            .map(|cap| {
                if let Some(sub) = cap.get(1) {
                    //println!("cap.get(1): {}", sub.as_str());
                    RuleChild::from_nt(sub.as_str(), ctx)
                } else if let Some(sub) = cap.get(2) {
                    //println!("String: {}, cap.get(2): {}", format, sub.as_str());
                    //println!("String: {}, cap.get(2): {}", format, sub.as_str().replace("\\{", "{").replace("\\}", "}"));
                    RuleChild::from_lit(&(sub.as_str().replace("\\{", "{").replace("\\}", "}")))
                } else {
                    unreachable!()
                }
            })
            .collect::<Vec<_>>();
    }

    pub fn unparse<W: Write, T: TreeLike>(
        &self,
        tree: &T,
        mut id: NodeID,
        ctx: &Context,
        w: &mut W,
    ) -> Result<NodeID, Error> {
        for child in self.children.iter() {
            id = child.unparse(tree, id, ctx, w)?;
        }
        return Ok(id);
    }

    pub fn nonterms(&self) -> &Vec<NTermID> {
        return &self.nonterms;
    }
    pub fn children(&self) -> &Vec<RuleChild> {
        return &self.children;
    }

    pub fn number_of_nonterms(&self) -> usize {
        return self.nonterms.len();
    }

    pub fn nonterm(&self) -> NTermID {
        return self.nonterm;
    }

    pub fn generate(&self, tree: &mut Tree, ctx: &Context, len: usize) -> usize {
        // println!("Rhs: {:?}, len: {}", self.nonterms, len);
        // println!("Min needed len: {}", self.nonterms.iter().fold(0, |sum, nt| sum + ctx.get_min_len_for_nt(*nt) ));
        let minimal_needed_len = self
            .nonterms
            .iter()
            .fold(0, |sum, nt| sum + ctx.get_min_len_for_nt(*nt));
        assert!(minimal_needed_len <= len);
        let mut remaining_len = len;
        if ctx.is_dumb() {
            remaining_len -= minimal_needed_len;
        }

        //if we have no further children, we consumed no len
        let mut total_size = 1;
        let paren = NodeID::from(tree.rules.len() - 1);
        //generate each childs tree from the left to the right. That way the only operation we ever
        //perform is to push another node to the end of the tree_vec

        for (i, nt) in self.nonterms.iter().enumerate() {
            //sample how much len this child can use up (e.g. how big can
            //let cur_child_max_len = Rule::get_random_len(remaining_nts, remaining_len) + ctx.get_min_len_for_nt(*nt);
            let mut cur_child_max_len;
            let mut new_nterms = Vec::new();
            new_nterms.extend_from_slice(&self.nonterms[i..]);
            if new_nterms.len() != 0 {
                cur_child_max_len = ctx.get_random_len(remaining_len, &new_nterms);
            } else {
                cur_child_max_len = remaining_len;
            }
            if ctx.is_dumb() {
                cur_child_max_len += ctx.get_min_len_for_nt(*nt);
            }

            //get a rule that can be used with the remaining length
            let rid = ctx.get_random_rule_for_nt(*nt, cur_child_max_len);
            assert!(
                ctx.is_dumb()
                    || !ctx
                        .get_possibilities_for_rule(
                            ctx.get_rule(rid).nonterms(),
                            cur_child_max_len - 1
                        )
                        .is_zero()
            );
            assert_eq!(tree.rules.len(), tree.sizes.len());
            assert_eq!(tree.sizes.len(), tree.paren.len());
            let offset = tree.rules.len();

            tree.rules.push(NormalOrCustomRule::NormalRule(rid));
            tree.sizes.push(0);
            tree.paren.push(NodeID::from(0));

            //generate the subtree for this rule, return the total consumed len
            let consumed_len = ctx.get_rule(rid).generate(tree, ctx, cur_child_max_len - 1);
            tree.sizes[offset] = consumed_len;
            tree.paren[offset] = paren;

            //println!("{}: min_needed_len: {}, Min-len: {} Consumed len: {} cur_child_max_len: {} remaining len: {}, total_size: {}, len: {}", ctx.nt_id_to_s(nt.clone()), minimal_needed_len, ctx.get_min_len_for_nt(*nt), consumed_len, cur_child_max_len, remaining_len, total_size, len);
            assert!(consumed_len <= cur_child_max_len);

            //println!("Rule: {}, min_len: {}", ctx.nt_id_to_s(nt.clone()), ctx.get_min_len_for_nt(*nt));
            assert!(consumed_len >= ctx.get_min_len_for_nt(*nt));

            //we can use the len that where not consumed by this iteration during the next iterations,
            //therefore it will be redistributed evenly amongst the other
            if ctx.is_dumb() {
                remaining_len += ctx.get_min_len_for_nt(*nt);
            }
            remaining_len -= consumed_len;
            //add the consumed len to the total_len
            total_size += consumed_len;
        }
        //println!("Rule: {}, Size: {}", ctx.nt_id_to_s(self.nonterm.clone()), total_size);
        return total_size;
    }
}
