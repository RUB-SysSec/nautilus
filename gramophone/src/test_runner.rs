#![feature(vec_remove_item)]
extern crate antlr_parser;
extern crate forksrv;
extern crate grammartec;
extern crate serde_json;
extern crate time as othertime;
#[macro_use]
extern crate serde_derive;
extern crate argparse;
extern crate ron;

mod config;
mod fuzzer;
mod queue;
mod rules;
mod shared_state;
mod state;

use std::collections::HashMap;

use config::Config;
use forksrv::error::SubprocessError;
use fuzzer::Fuzzer;
use grammartec::chunkstore::ChunkStoreWrapper;
use grammartec::context::{Context, SerializableContext};
use queue::{InputState, QueueItem};
use shared_state::GlobalSharedState;
use state::FuzzingState;

use argparse::{ArgumentParser, Store, StoreTrue};
use othertime::strftime;
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::{thread, time};

fn main() {
    //Parse parameters
    let mut config_file_path = "config.ron".to_string();
    let mut input_path = "".to_string();
    let mut dumb = false;
    {
        // this block limits scope of borrows by ap.refer() method
        let mut ap = ArgumentParser::new();
        ap.set_description("Target Tester");
        ap.refer(&mut config_file_path)
            .add_option(&["-g"], Store, "Path to config file. Default ./config.ron")
            .metavar("config_file");
        ap.refer(&mut input_path)
            .add_argument("input_path", Store, "Path to the input to test")
            .required();
        ap.parse_args_or_exit();
    }

    //Set Config
    let mut config_file = File::open(&config_file_path).expect("cannot read config file");
    let mut config_file_contents = String::new();
    config_file
        .read_to_string(&mut config_file_contents)
        .expect("RAND_1413661228");
    let config: Config = ron::de::from_str(&config_file_contents).expect("Failed to deserialize");


    let path_to_bin_target = config.path_to_bin_target.to_owned();
    let args = config.arguments.clone();

    let global_state = Arc::new(Mutex::new(GlobalSharedState::new(
        config.path_to_workdir.clone(),
    )));

    let mut fuzzer = Fuzzer::new(
        path_to_bin_target.clone(),
        args,
        global_state.clone(),
        config.dump_mode,
        config.path_to_workdir.clone(),
    ).expect("RAND_3617502350");


    let mut inputs : Vec<(usize, Vec<u8>, Vec<u8>)>= vec!();
    #[cfg(assert_debug)]
    for i in (1..376){
        let mut input_file = File::open(&format!("/tmp/debug_backup/run_data_{}",i)).expect("cannot open input file");
        let mut input_data = vec!();
        input_file.read_to_end(&mut input_data).expect("cannot read input file");

        let (exit_reason,time) = fuzzer.exec_raw(&input_data).expect("failed to run on testcase");
        let bitmap = fuzzer.last_bitmap().to_vec();
        let mut h = DefaultHasher::new();
        bitmap.hash(&mut h);
        println!("loading {}, hash = {}",i, h.finish());
        inputs.push( (i,input_data, bitmap ) )
    }

    for i in (1..5000){
        let x = i*(i^0xc2266325d0886cc5)*0x6b81e593fe83f44e + i;
        println!("testing {}",x%inputs.len());
        let (id, ref input_data, ref bitmap) = inputs[x%inputs.len()];
        let (exit_reason,time) = fuzzer.exec_raw(&input_data).expect("failed to run on testcase");
        assert_eq!(bitmap, &fuzzer.last_bitmap().to_vec());
    }
}
