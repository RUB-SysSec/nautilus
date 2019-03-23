use num::CheckedAdd;
use num::{ToPrimitive, Zero};
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;

use loaded_dice::LoadedDiceSampler;
use rand::{sample, thread_rng, Rng, StdRng};

use newtypes::{NTermID, RuleID};
use rule::Rule;
use tree::Tree;

#[derive(Clone)]
pub struct Context {
    rules: Vec<Rule>,
    nts_to_rules: HashMap<NTermID, Vec<RuleID>>,
    nt_ids_to_name: HashMap<NTermID, String>,
    names_to_nt_id: HashMap<String, NTermID>,

    rules_to_min_size: HashMap<RuleID, usize>,
    nts_to_min_size: HashMap<NTermID, usize>,
    nts_to_rule_samplers: HashMap<NTermID, Vec<Option<RefCell<LoadedDiceSampler<StdRng>>>>>,
    nts_to_len_samplers: HashMap<NTermID, RefCell<LoadedDiceSampler<StdRng>>>,
    nt_and_n_to_count: HashMap<(NTermID, usize), u16>,
    rhs_and_n_to_count: HashMap<(Vec<NTermID>, usize), u16>,
    rhs_and_n_to_count_u32: HashMap<(Vec<NTermID>, usize), u32>,
    rule_id_to_possible_lens: HashMap<RuleID, Vec<usize>>,
    max_len: usize,
    dumb: bool,
}

#[derive(Serialize, Deserialize)]
pub struct SerializableContext {
    rules: Vec<Rule>,
    nts_to_rules: HashMap<NTermID, Vec<RuleID>>,
    nt_ids_to_name: HashMap<NTermID, String>,
    names_to_nt_id: HashMap<String, NTermID>,
    rules_to_min_size: HashMap<RuleID, usize>,
    nts_to_min_size: HashMap<NTermID, usize>,
    nt_and_n_to_count: HashMap<(NTermID, usize), u16>,
    rhs_and_n_to_count: HashMap<(Vec<NTermID>, usize), u16>,
    rhs_and_n_to_count_u32: HashMap<(Vec<NTermID>, usize), u32>,
    rule_id_to_possible_lens: HashMap<RuleID, Vec<usize>>,
    max_len: usize,
    pub hash_of_original: u64,
    pub dumb: bool,
}

impl Context {
    pub fn new() -> Self {
        Self::with_dump(false)
    }

    pub fn with_dump(dumb: bool) -> Self {
        return Context {
            rules: vec![],
            nts_to_rules: HashMap::new(),
            nt_ids_to_name: HashMap::new(),
            names_to_nt_id: HashMap::new(),
            rules_to_min_size: HashMap::new(),
            nts_to_min_size: HashMap::new(),
            nts_to_rule_samplers: HashMap::new(),
            nts_to_len_samplers: HashMap::new(),
            rhs_and_n_to_count_u32: HashMap::new(),
            nt_and_n_to_count: HashMap::new(),
            rhs_and_n_to_count: HashMap::new(),
            rule_id_to_possible_lens: HashMap::new(),
            max_len: 0,
            dumb,
        };
    }

    pub fn initialize(&mut self, max_len: usize, verbose: bool) {
        self.calc_min_len();
        self.max_len = max_len + 2;
        if !self.dumb {
            self.calc_sampler(max_len, verbose);
            self.set_rule_id_to_possible_lengths();
        }
    }

    pub fn create_serializable_context(&self, hash_of_original: u64) -> SerializableContext {
        return SerializableContext {
            rules: self.rules.clone(),
            nts_to_rules: self.nts_to_rules.clone(),
            nt_ids_to_name: self.nt_ids_to_name.clone(),
            names_to_nt_id: self.names_to_nt_id.clone(),
            rules_to_min_size: self.rules_to_min_size.clone(),
            nts_to_min_size: self.nts_to_min_size.clone(),
            rhs_and_n_to_count_u32: self.rhs_and_n_to_count_u32.clone(),
            nt_and_n_to_count: self.nt_and_n_to_count.clone(),
            rhs_and_n_to_count: self.rhs_and_n_to_count.clone(),
            rule_id_to_possible_lens: self.rule_id_to_possible_lens.clone(),
            max_len: self.max_len,
            hash_of_original,
            dumb: self.dumb,
        };
    }

    pub fn from_serialized_context(
        saved_context: SerializableContext,
        verbose: bool,
        dumb: bool,
    ) -> Self {
        let max_len = saved_context.max_len;
        let mut context = Context {
            rules: saved_context.rules,
            nts_to_rules: saved_context.nts_to_rules,
            nt_ids_to_name: saved_context.nt_ids_to_name,
            names_to_nt_id: saved_context.names_to_nt_id,
            rules_to_min_size: saved_context.rules_to_min_size,
            nts_to_min_size: saved_context.nts_to_min_size,
            nts_to_rule_samplers: HashMap::new(),
            nts_to_len_samplers: HashMap::new(),
            rhs_and_n_to_count_u32: saved_context.rhs_and_n_to_count_u32,
            nt_and_n_to_count: saved_context.nt_and_n_to_count,
            rhs_and_n_to_count: saved_context.rhs_and_n_to_count,
            rule_id_to_possible_lens: saved_context.rule_id_to_possible_lens,
            max_len,
            dumb,
        };
        if !dumb {
            context.calc_sampler(max_len - 2, verbose);
            if saved_context.dumb && !dumb {
                context.set_rule_id_to_possible_lengths();
            }
        }
        return context;
    }

    pub fn get_rule(&self, r: RuleID) -> &Rule {
        let id: usize = r.into();
        return &self.rules[id];
    }

    pub fn get_nt(&self, r: RuleID) -> NTermID {
        return self.get_rule(r).nonterm();
    }

    pub fn get_num_children(&self, r: RuleID) -> usize{
        return self.get_rule(r).number_of_nonterms();
    }

    pub fn add_rule(&mut self, nt: &str, format: &str) -> RuleID {
        let rid = self.rules.len().into();
        let rule = Rule::from_format(self, nt, format);
        let ntid = self.aquire_nt_id(nt);
        self.rules.push(rule);
        self.nts_to_rules
            .entry(ntid)
            .or_insert_with(|| vec![])
            .push(rid);
        return rid;
    }

    pub fn add_term_rule(&mut self, nt: &str, term: &Vec<u8>) -> RuleID {
        let rid = self.rules.len().into();
        let ntid = self.aquire_nt_id(nt);
        self.rules.push(Rule::from_term(ntid, term));
        self.nts_to_rules
            .entry(ntid)
            .or_insert_with(|| vec![])
            .push(rid);
        return rid;
    }

    pub fn aquire_nt_id(&mut self, nt: &str) -> NTermID {
        let next_id = self.nt_ids_to_name.len().into();
        let id = self.names_to_nt_id.entry(nt.into()).or_insert(next_id);
        self.nt_ids_to_name.entry(*id).or_insert(nt.into());
        return *id;
    }

    pub fn is_dumb(&self) -> bool {
        return self.dumb;
    }

    pub fn nt_id(&self, nt: &str) -> NTermID {
        return *self
            .names_to_nt_id
            .get(nt)
            .expect(&("no such nonterminal: ".to_owned() + nt));
    }

    pub fn nt_id_to_s(&self, nt: NTermID) -> String {
        return self.nt_ids_to_name[&nt].clone();
    }

    fn calc_min_len_for_rule(&self, r: RuleID) -> Option<usize> {
        let mut res = 1;
        for nt_id in self.get_rule(r).nonterms().iter() {
            if let Some(min) = self.nts_to_min_size.get(nt_id) {
                //println!("Calculating length for Rule(calc_min_len_for_rule): {}, current: {}, adding: {}, because of rule: {}", self.nt_id_to_s(self.get_rule(r).nonterm().clone()), res, min, self.nt_id_to_s(nt_id.clone()));
                res += *min;
            } else {
                return None;
            }
        }
        //println!("Calculated length for Rule(calc_min_len_for_rule): {}, Length: {}", self.nt_id_to_s(self.get_rule(r).nonterm().clone()), res);
        return Some(res);
    }

    pub fn calc_min_len(&mut self) {
        let mut something_changed = true;
        while something_changed == true {
            //TODO: find a better solution to prevent  consumed_len >= ctx.get_min_len_for_nt(*nt)' Assertions
            let mut unknown_rules = (0..self.rules.len())
                .map(|i| RuleID::from(i))
                .collect::<Vec<_>>();
            something_changed = false;
            while unknown_rules.len() > 0 {
                let last_len = unknown_rules.len();
                unknown_rules.retain(|rule| {
                    if let Some(min) = self.calc_min_len_for_rule(*rule) {
                        let nt = self.get_rule(*rule).nonterm();
                        //let name = self.nt_id_to_s(nt.clone()); //DEBUGGING
                        let e = self.nts_to_min_size.entry(nt).or_insert(min);
                        if *e > min {
                            *e = min;
                            something_changed = true;
                        }
                        //println!("Calculated length for Rule: {}, Length: {}, Min_length_of_nt: {}", name, min, *e);
                        self.rules_to_min_size.insert(*rule, min);
                        false
                    } else {
                        true
                    }
                });
                if last_len == unknown_rules.len() {
                    for i in 0..self.nt_ids_to_name.len() {
                        println!(
                            "NTermID {} = {}",
                            i,
                            self.nt_ids_to_name
                                .get(&NTermID::from(i))
                                .expect("RAND_770212435")
                        );
                    }
                    panic!(format!(
                        "unproductive rules: {:?}",
                        unknown_rules
                            .iter()
                            .map(|r| self.get_rule(*r))
                            .collect::<Vec<_>>()
                    ));
                }
            }
        }
        self.calc_rule_order();
    }

    fn calc_rule_order(&mut self) {
        let rules_to_min_size = &self.rules_to_min_size;
        for rules in self.nts_to_rules.values_mut() {
            (*rules).sort_by(|r1, r2| rules_to_min_size[r1].cmp(&rules_to_min_size[r2]));
        }
    }

    fn calc_sampler(&mut self, max_len: usize, verbose: bool) {
        //Check if all min_lens are less than max_len
        for min_size in self.rules_to_min_size.values() {
            assert!(*min_size < self.max_len);
        }

        //Get set of all nterms
        let mut nterms = HashSet::new();
        for rule in self.rules.iter() {
            nterms.insert(rule.nonterm());
        }

        //Initialize HashMaps for all nterms
        for nterm in nterms.iter() {
            self.nts_to_rule_samplers
                .insert(nterm.clone(), vec![None; self.max_len]);
        }

        //Calculate subtrees
        if verbose {
            print!("Calculating possible subtrees:");
        }
        for i in 1..self.max_len {
            for nterm in nterms.iter() {
                self.count_possibilities_nterm(&nterm, i);

                //create rule sampler
                if self
                    .nt_and_n_to_count
                    .get(&(*nterm, i))
                    .expect("RAND_2374448501") != &0
                {
                    let mut norm_factor: u64 = 0;
                    let rules_for_nt = self
                        .nts_to_rules
                        .get(&nterm)
                        .expect("RAND_2561305800")
                        .clone();
                    let mut rule_probabilities: Vec<f64> = vec![0.0; rules_for_nt.len()];
                    for (x, rule_id) in rules_for_nt.iter().enumerate() {
                        let nterms = self.get_rule(rule_id.clone()).nonterms().clone();
                        rule_probabilities[x] +=
                            self.count_possibilities_rule(&nterms, i - 1)
                                .to_u32()
                                .unwrap_or(u32::max_value()) as f64;
                        norm_factor +=
                            self.count_possibilities_rule(&nterms, i - 1)
                                .to_u32()
                                .unwrap_or(u32::max_value()) as u64;
                    }
                    for x in 0..rule_probabilities.len() {
                        rule_probabilities[x] /= norm_factor as f64;
                    }
                    // println!("Sampler: Nterm: {};\tDepth: {};\t\tRule probabilities: {:?}", self.nt_ids_to_name.get(&nterm).expect("RAND_1038242446"), i, rule_probabilities);
                    let sampler = LoadedDiceSampler::new(
                        rule_probabilities,
                        StdRng::new().expect("RAND_2910855259"),
                    );
                    self.nts_to_rule_samplers
                        .get_mut(&nterm)
                        .expect("RAND_1458598779")[i] = Some(RefCell::new(sampler));
                }
            }
            if verbose {
                print!(
                    "\rCalculating possible subtrees: {}%",
                    (i * 100) / (self.max_len - 1)
                );
            }
        }
        for nterm in nterms.iter() {
            let mut probabilities = vec![0.0; self.max_len];
            let mut norm_factor: u64 = 0;
            for i in 1..self.max_len {
                let p = self.get_possibilities_for_nterm(nterm, i);
                norm_factor += p as u64;
                probabilities[i] = p as f64;
            }
            for i in 1..self.max_len {
                probabilities[i] /= norm_factor as f64;
            }
            // println!("Len Sampler: Nterm: {};\tLen probabilities: {:?}", self.nt_ids_to_name.get(&nterm).expect("RAND_3680791943"), probabilities);
            let sampler =
                LoadedDiceSampler::new(probabilities, StdRng::new().expect("RAND_2190718084"));
            self.nts_to_len_samplers
                .insert(nterm.clone(), RefCell::new(sampler));
        }
        if verbose {
            print!("\n");
        }

        // for nterm in  nterms.iter() {
        //     for i in 1..max_len {
        //         println!("Nterm: {}(NtermID: {})\tLen: {}\tPossible Subtrees: {:?}", self.nt_ids_to_name.get(&nterm).expect("RAND_3444108000"), self.nt_ids_to_name[&nterm], i, self.nt_and_n_to_count.get(&(*nterm, i)).expect("RAND_3444108000").to_u64().unwrap_or(u64::max_value()));
        //     }
        // }
        // for (key, possibilities) in self.rhs_and_n_to_count.iter() {
        //     println!("Nonterms and len: {:?}\t\tNumber of Subrees: {:?}\t\tExact: {:?}", key, possibilities.to_u64().unwrap_or(u64::max_value()), possibilities);
        // }
        // println!("{:?}", self.nt_ids_to_name);
    }

    fn set_rule_id_to_possible_lengths(&mut self) {
        for rule_id in self.rules_to_min_size.keys() {
            let mut lengths = Vec::new();
            for i in 1..self.max_len {
                if !self
                    .get_possibilities_for_rule(self.rules[rule_id.to_i()].nonterms(), i - 1)
                    .is_zero()
                {
                    lengths.push(i);
                }
            }
            self.rule_id_to_possible_lens
                .insert(rule_id.clone(), lengths.clone());
        }
    }

    fn count_possibilities_nterm(&mut self, nt: &NTermID, len: usize) -> u16 {
        if len < 1 {
            return 0;
        }
        if self.nt_and_n_to_count.contains_key(&(*nt, len)) {
            return self
                .nt_and_n_to_count
                .get(&(*nt, len))
                .expect("RAND_1364312715")
                .clone();
        }
        let mut sum = 0;
        let rules = self.nts_to_rules.get(&nt).expect("RAND_3987216527").clone();
        for rule_id in rules.iter() {
            let nterms = self.get_rule(rule_id.clone()).nonterms().clone();
            match sum.checked_add(&self.count_possibilities_rule(&nterms, len - 1)) {
                Some(x) => sum = x,
                None => {
                    sum = u16::max_value();
                    break;
                }
            }
        }
        self.nt_and_n_to_count.insert((*nt, len), sum);
        return sum;
    }

    fn count_possibilities_rule(&mut self, nterms: &Vec<NTermID>, len: usize) -> u16 {
        if nterms.len() == 0 {
            return if len == 0 { 1 } else { 0 };
        }
        if self.rhs_and_n_to_count.contains_key(&(nterms.clone(), len)) {
            return self
                .rhs_and_n_to_count
                .get(&(nterms.clone(), len))
                .expect("RAND_4255885820")
                .clone();
        }
        let mut possibilities: u32 = 0;
        let mut new_nterms = Vec::new();
        new_nterms.extend_from_slice(&nterms[1..]);
        for s in 0..len + 1 {
            possibilities += self
                .count_possibilities_rule(&new_nterms, s)
                .checked_mul(self.count_possibilities_nterm(&nterms[0], len - s))
                .unwrap_or(u16::max_value()) as u32;
        }

        let res: u16 = if possibilities > u16::max_value() as u32 {
            u16::max_value()
        } else {
            possibilities as u16
        };
        self.rhs_and_n_to_count_u32
            .insert((nterms.clone(), len), possibilities);
        self.rhs_and_n_to_count.insert((nterms.clone(), len), res);
        return res;
    }

    pub fn get_possibilities_for_rule(&self, nterms: &Vec<NTermID>, len: usize) -> u16 {
        if nterms.len() == 0 {
            return if len == 0 { 1 } else { 0 };
        }
        return self
            .rhs_and_n_to_count
            .get(&(nterms.clone(), len))
            .expect("RAND_3786858109")
            .clone();
    }

    pub fn get_possibilities_for_rule_u32(&self, nterms: &Vec<NTermID>, len: usize) -> u32 {
        if nterms.len() == 0 {
            return if len == 0 { 1 } else { 0 };
        }
        return self
            .rhs_and_n_to_count_u32
            .get(&(nterms.clone(), len))
            .expect("RAND_3519890687")
            .clone();
    }

    fn get_possibilities_for_nterm(&self, nt: &NTermID, len: usize) -> u16 {
        if len < 1 {
            return 0;
        }
        return self
            .nt_and_n_to_count
            .get(&(*nt, len))
            .expect("RAND_3164178569")
            .clone();
    }

    pub fn check_if_nterm_has_multiple_possiblities(&self, nt: &NTermID) -> bool {
        if self.dumb {
            return self.get_rules_for_nt(*nt).len() > 1;
        }
        let mut counter = 0;
        for i in 1..self.max_len {
            counter = counter
                .checked_add(&self.get_possibilities_for_nterm(nt, i))
                .unwrap_or(2);
            if counter > 1 {
                return true;
            }
        }
        return false;
    }

    pub fn get_random_len(&self, len: usize, rhs_of_rule: &Vec<NTermID>) -> usize {
        if self.dumb {
            return self.dumb_get_random_len(rhs_of_rule.len(), len);
        }
        let possibilities = self.get_possibilities_for_rule_u32(rhs_of_rule, len);
        assert_ne!(possibilities, 0);
        let mut counter = 0;
        let mut remaining_nts = Vec::new();
        remaining_nts.extend_from_slice(&rhs_of_rule[1..]);
        let nt = &rhs_of_rule[0];
        let random = thread_rng().gen_range(0, possibilities);
        for i in 0..len + 1 {
            counter += self
                .get_possibilities_for_rule(&remaining_nts.to_vec(), i)
                .checked_mul(self.get_possibilities_for_nterm(nt, len - i))
                .unwrap_or(u16::max_value()) as u32;
            if counter > random {
                return len - i;
            };
        }
        println!(
            "counter: {:?}, nterms: {:?}, random: {:?}, possibilities: {:?}",
            counter, remaining_nts, random, possibilities
        );
        panic!(
            "No random len for {} within {} steps found!",
            self.nt_ids_to_name[&nt], len
        )
    }

    //we need to get maximal sizes for all subtrees. To generate trees fairly, we want to split the
    //available size fairly to all nodes. (e.g. all children have the same expected size,
    //regardless of its index in the current rule. We use this version of the algorithm described
    //here: https://stackoverflow.com/a/8068956 to get the first value.
    fn dumb_get_random_len(&self, number_of_children: usize, total_remaining_len: usize) -> usize {
        let mut res = total_remaining_len;
        let iters = (number_of_children as i32) - 1;
        for _ in 0..iters {
            let proposal = thread_rng().gen_range(0, total_remaining_len + 1);
            if proposal < res {
                res = proposal
            }
        }
        return res;
    }

    pub fn get_min_len_for_nt(&self, nt: NTermID) -> usize {
        return self.nts_to_min_size[&nt];
    }

    pub fn get_random_rule_for_nt(&self, nt: NTermID, len: usize) -> RuleID {
        if self.dumb {
            return self.dumb_get_random_rule_for_nt(nt, len);
        }
        //println!("Deriving {} within {} steps", self.nt_ids_to_name[&nt], len);
        match self.nts_to_rule_samplers.get(&nt).expect("RAND_4063454361")[len] {
            Some(ref sampler) => {
                let rule_id = self.nts_to_rules.get(&nt).expect("RAND_3202426897")
                    [sampler.borrow_mut().sample()];
                assert!(
                    !self
                        .get_possibilities_for_rule(
                            self.get_rule(rule_id.clone()).nonterms(),
                            len - 1
                        )
                        .is_zero()
                );
                return rule_id;
            }
            None => panic!(
                "there is no way to derive {} within {} steps",
                self.nt_ids_to_name[&nt], len
            ),
        }
    }

    fn dumb_get_random_rule_for_nt(&self, nt: NTermID, max_len: usize) -> RuleID {
        let applicable_rules = self.nts_to_rules[&nt]
            .iter()
            .take_while(|r| self.rules_to_min_size[r] <= max_len);
        match sample(&mut thread_rng(), applicable_rules, 1).pop() {
            Some(rule) => return *rule,
            None => panic!(
                "there is no way to derive {} within {} steps",
                self.nt_ids_to_name[&nt], max_len
            ),
        }
    }

    pub fn get_random_len_for_ruleid(&self, rule_id: &RuleID) -> usize {
        return *thread_rng()
            .choose(
                &self
                    .rule_id_to_possible_lens
                    .get(rule_id)
                    .expect("RAND_1390563191"),
            )
            .expect("RAND_852077306") - 1;
    }

    pub fn get_random_len_for_nt(&self, nt: &NTermID) -> usize {
        if self.dumb {
            return self.max_len;
        }
        return self
            .nts_to_len_samplers
            .get(&nt)
            .expect("RAND_2940771921")
            .borrow_mut()
            .sample();
    }

    pub fn get_rules_for_nt(&self, nt: NTermID) -> &Vec<RuleID> {
        return &self.nts_to_rules[&nt];
    }

    pub fn generate_tree_from_nt(&self, nt: NTermID, max_len: usize) -> Tree {
        return self.generate_tree_from_rule(self.get_random_rule_for_nt(nt, max_len), max_len - 1);
    }

    pub fn generate_tree_from_rule(&self, r: RuleID, len: usize) -> Tree {
        let mut tree = Tree::from_rule_vec(vec![], self);
        // println!("Rule: {}, len: {}, nonterms: {:?}", self.nt_ids_to_name.get(&self.get_rule(r.clone()).nonterm()).expect("RAND_3800709163"), max_len, self.get_rule(r.clone()).nonterms());
        assert!(
            self.dumb
                || !self
                    .get_possibilities_for_rule(self.get_rule(r.clone()).nonterms(), len)
                    .is_zero()
        );
        tree.generate_from_rule(r, len, self);
        return tree;
    }
}

#[cfg(test)]
mod tests {
    use context::Context;
    use newtypes::RuleID;
    use rule::{NormalOrCustomRule, Rule, RuleChild};
    use std::collections::HashSet;
    use tree::{Tree, TreeLike};

    #[test]
    fn simple_context() {
        let mut ctx = Context::new();
        let r = Rule::from_format(&mut ctx, "F", "foo{A:a}\\{bar\\}{B:b}asd{C}");
        let soll = vec![
            RuleChild::from_lit("foo"),
            RuleChild::from_nt("{A:a}", &mut ctx),
            RuleChild::from_lit("{bar}"),
            RuleChild::from_nt("{B:b}", &mut ctx),
            RuleChild::from_lit("asd"),
            RuleChild::from_nt("{C}", &mut ctx),
        ];
        assert_eq!(r.children(), &soll);
        assert_eq!(r.nonterms()[0], ctx.nt_id("A"));
        assert_eq!(r.nonterms()[1], ctx.nt_id("B"));
        assert_eq!(r.nonterms()[2], ctx.nt_id("C"));
    }

    #[test]
    fn test_context() {
        let mut ctx = Context::new();
        let r0 = ctx.add_rule("C", "c{B}c");
        let r1 = ctx.add_rule("B", "b{A}b");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let r3 = ctx.add_rule("A", "a");
        ctx.initialize(5, false);
        assert_eq!(ctx.get_min_len_for_nt(ctx.nt_id("A")), 1);
        assert_eq!(ctx.get_min_len_for_nt(ctx.nt_id("B")), 2);
        assert_eq!(ctx.get_min_len_for_nt(ctx.nt_id("C")), 3);
        let mut tree = Tree::from_rule_vec(vec![], &ctx);
        tree.generate_from_nt(ctx.nt_id("C"), 3, &ctx);
        assert_eq!(
            tree.rules,
            vec![
                NormalOrCustomRule::NormalRule(r0),
                NormalOrCustomRule::NormalRule(r1),
                NormalOrCustomRule::NormalRule(r3),
            ]
        );
        let mut data: Vec<u8> = vec![];
        tree.unparse_to(&ctx, &mut data).expect("RAND_498873613");
        assert_eq!(String::from_utf8(data).expect("RAND_3377050372"), "cbabc");
    }

    #[test]
    fn get_random_len_for_ruleid() {
        let mut ctx = Context::new();
        let _ = ctx.add_rule("C", "c{B}c");
        let _ = ctx.add_rule("B", "b{D}b");
        let _ = ctx.add_rule("B", "b");
        let _ = ctx.add_rule("D", "{B}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {B}");
        let _ = ctx.add_rule("A", "a");
        ctx.initialize(10, false);
        let mut lens_for_a = HashSet::new();
        let mut lens_for_b = HashSet::new();
        for _ in 0..100 {
            lens_for_a.insert(ctx.get_random_len_for_ruleid(&RuleID::from(4)));
            lens_for_b.insert(ctx.get_random_len_for_ruleid(&RuleID::from(1)));
        }
        assert!(lens_for_a.contains(&1));
        assert!(lens_for_a.contains(&2));
        assert!(lens_for_a.contains(&3));
        assert!(lens_for_a.contains(&4));
        assert!(lens_for_a.contains(&5));
        assert!(lens_for_a.contains(&6));
        assert!(lens_for_a.contains(&7));
        assert!(lens_for_a.contains(&8));
        assert!(lens_for_a.contains(&9));
        assert!(lens_for_a.contains(&10));
        assert!(!lens_for_a.contains(&11));

        assert!(lens_for_b.contains(&2));
        assert!(lens_for_b.contains(&4));
        assert!(lens_for_b.contains(&6));
        assert!(lens_for_b.contains(&8));
        assert!(lens_for_b.contains(&10));
        assert!(!lens_for_b.contains(&1));
        assert!(!lens_for_b.contains(&3));
        assert!(!lens_for_b.contains(&5));
        assert!(!lens_for_b.contains(&7));
        assert!(!lens_for_b.contains(&9));
        assert!(!lens_for_a.contains(&11));
    }

    #[test]
    fn test_generate_len() {
        let mut ctx = Context::new();
        let r0 = ctx.add_rule("E", "({E}+{E})");
        let r1 = ctx.add_rule("E", "({E}*{E})");
        let r2 = ctx.add_rule("E", "({E}-{E})");
        let r3 = ctx.add_rule("E", "({E}/{E})");
        let r4 = ctx.add_rule("E", "1");
        ctx.initialize(11, false);
        assert_eq!(ctx.get_min_len_for_nt(ctx.nt_id("E")), 1);

        for _ in 0..100 {
            let mut tree = Tree::from_rule_vec(vec![], &ctx);
            tree.generate_from_nt(ctx.nt_id("E"), 9, &ctx);
            assert!(tree.rules.len() < 10);
            assert!(tree.rules.len() >= 1);
        }

        let rules = vec![
            NormalOrCustomRule::NormalRule(r0),
            NormalOrCustomRule::NormalRule(r1),
            NormalOrCustomRule::NormalRule(r4),
            NormalOrCustomRule::NormalRule(r4),
            NormalOrCustomRule::NormalRule(r4),
        ];
        let tree = Tree::from_rule_vec(rules, &ctx);
        let mut data: Vec<u8> = vec![];
        tree.unparse_to(&ctx, &mut data).expect("RAND_2530190768");
        assert_eq!(
            String::from_utf8(data).expect("RAND_3492562908"),
            "((1*1)+1)"
        );

        let rules = vec![
            NormalOrCustomRule::NormalRule(r0),
            NormalOrCustomRule::NormalRule(r1),
            NormalOrCustomRule::NormalRule(r2),
            NormalOrCustomRule::NormalRule(r3),
            NormalOrCustomRule::NormalRule(r4),
            NormalOrCustomRule::NormalRule(r4),
            NormalOrCustomRule::NormalRule(r4),
            NormalOrCustomRule::NormalRule(r4),
            NormalOrCustomRule::NormalRule(r4),
        ];
        let tree = Tree::from_rule_vec(rules, &ctx);
        let mut data: Vec<u8> = vec![];
        tree.unparse_to(&ctx, &mut data).expect("RAND_3988925787");
        assert_eq!(
            String::from_utf8(data).expect("RAND_4245419893"),
            "((((1/1)-1)*1)+1)"
        );
    }

    #[test]
    fn test_context_serialization() {
        let mut ctx = Context::new();
        let _ = ctx.add_rule("C", "c{B}c");
        let _ = ctx.add_rule("B", "b{D}b");
        let _ = ctx.add_rule("B", "b");
        let _ = ctx.add_rule("D", "{B}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {A}");
        let _ = ctx.add_rule("A", "a {B}");
        let _ = ctx.add_rule("A", "a");
        ctx.initialize(10, false);

        let serial_ctx = ctx.create_serializable_context(1);
        let ctx2 = Context::from_serialized_context(serial_ctx, false, false);
        assert_eq!(ctx.rules, ctx2.rules);
        assert_eq!(ctx.nts_to_rules, ctx2.nts_to_rules);
        assert_eq!(ctx.nt_ids_to_name, ctx2.nt_ids_to_name);
        assert_eq!(ctx.names_to_nt_id, ctx2.names_to_nt_id);
        assert_eq!(ctx.rules_to_min_size, ctx2.rules_to_min_size);
        assert_eq!(ctx.nts_to_min_size, ctx2.nts_to_min_size);
        assert_eq!(ctx.nt_and_n_to_count, ctx2.nt_and_n_to_count);
        assert_eq!(ctx.rhs_and_n_to_count, ctx2.rhs_and_n_to_count);
        assert_eq!(ctx.rhs_and_n_to_count_u32, ctx2.rhs_and_n_to_count_u32);
        assert_eq!(ctx.rule_id_to_possible_lens, ctx2.rule_id_to_possible_lens);
        assert_eq!(ctx.max_len, ctx2.max_len);
    }
}
