use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io::ErrorKind;

use forksrv::exitreason::ExitReason;
use grammartec::context::Context;
use grammartec::newtypes::NodeID;
use grammartec::tree::Tree;
use grammartec::tree::TreeLike;

#[derive(Serialize, Clone, Deserialize)]
pub enum InputState {
    Init(usize),
    Det((usize, usize)),
    DetAFL(usize),
    Random,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: usize,
    pub tree: Tree,
    pub fresh_bits: HashSet<usize>,
    pub all_bits: Vec<u8>,
    pub exitreason: ExitReason,
    pub state: InputState,
    pub recursions: Option<Vec<(NodeID, NodeID)>>,
    pub execution_time: u32,
}

impl QueueItem {
    pub fn new(
        id: usize,
        tree: Tree,
        fresh_bits: HashSet<usize>,
        all_bits: Vec<u8>,
        exitreason: ExitReason,
        execution_time: u32,
    ) -> Self {
        return QueueItem {
            id,
            tree,
            fresh_bits,
            all_bits,
            exitreason,
            state: InputState::Init(0),
            recursions: None,
            execution_time,
        };
    }
}

#[derive(Serialize, Deserialize)]
pub struct Queue {
    pub inputs: Vec<QueueItem>,
    pub processed: Vec<QueueItem>,
    pub bit_to_inputs: HashMap<usize, Vec<usize>>,
    pub current_id: usize,
    pub work_dir: String,
}

impl Queue {
    pub fn add(
        &mut self,
        tree: Tree,
        all_bits: Vec<u8>,
        mut new_bits: Vec<usize>,
        exitreason: ExitReason,
        ctx: &Context,
        execution_time: u32,
    ) {
        if all_bits
            .iter()
            .enumerate()
            .all(|(i, elem)| (*elem == 0) || self.bit_to_inputs.contains_key(&i))
        {
            return;
        }
        let mut fresh_bits = HashSet::new();
        //Check which bits are new and insert them into fresh_bits
        for (i, elem) in all_bits.iter().enumerate() {
            if *elem != 0 {
                if !self.bit_to_inputs.contains_key(&i) {
                    fresh_bits.insert(i.clone());
                }
                self.bit_to_inputs
                    .entry(i)
                    .or_insert(vec![])
                    .push(self.current_id);
            }
        }

        //Create File for entry
        let mut file = File::create(format!(
            "{}outputs/queue/id:{:09},er:{:?}",
            self.work_dir, self.current_id, exitreason
        )).expect("RAND_259979732");
        tree.unparse_to(&ctx, &mut file).expect("RAND_3408190314");

        //Add entry to queue
        self.inputs.push(QueueItem::new(
            self.current_id,
            tree,
            fresh_bits,
            all_bits,
            exitreason,
            execution_time,
        ));

        //Increase current_id
        if self.current_id == usize::max_value() {
            self.current_id = 0;
        } else {
            self.current_id += 1;
        }
    }

    pub fn new(work_dir: String) -> Self {
        return Queue {
            inputs: vec![],
            processed: vec![],
            bit_to_inputs: HashMap::new(),
            current_id: 0,
            work_dir: work_dir,
        };
    }

    pub fn pop(&mut self) -> Option<QueueItem> {
        let option = self.inputs.pop();
        if option.is_some() {
            let item = option.expect("RAND_607640468");
            let id = item.id;
            let mut keys = Vec::with_capacity(self.bit_to_inputs.keys().len()); //TODO: Find a better solution for this
            {
                for k in self.bit_to_inputs.keys() {
                    keys.push(*k);
                }
            }
            for k in keys {
                let mut v = self.bit_to_inputs.remove(&k).expect("RAND_2593710501");
                v.remove_item(&id);
                if !v.is_empty() {
                    self.bit_to_inputs.insert(k, v);
                }
            }
            return Some(item);
        }
        return None;
    }

    pub fn finished(&mut self, item: QueueItem) {
        if item
            .all_bits
            .iter()
            .enumerate()
            .all(|(i, elem)| (*elem == 0) || self.bit_to_inputs.contains_key(&i))
        {
            //If file was created for this entry, delete it.
            match fs::remove_file(format!(
                "{}outputs/queue/id:{:09},er:{:?}",
                self.work_dir, item.id, item.exitreason
            )) {
                Err(ref err) if err.kind() != ErrorKind::NotFound => {
                    println!("Error while deleting file: {}", err);
                }
                _ => {}
            }
            return;
        }

        //Check which bits are new and insert them into fresh_bits
        let mut fresh_bits = HashSet::new();
        for (i, elem) in item.all_bits.iter().enumerate() {
            if *elem != 0 {
                if !self.bit_to_inputs.contains_key(&i) {
                    fresh_bits.insert(i.clone());
                }
                self.bit_to_inputs.entry(i).or_insert(vec![]).push(item.id);
            }
        }
        self.processed.push(item);
    }

    pub fn len(&self) -> usize {
        return self.inputs.len();
    }

    pub fn new_round(&mut self) {
        self.inputs.append(&mut self.processed);
    }
}
