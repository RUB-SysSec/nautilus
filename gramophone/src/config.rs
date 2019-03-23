pub const BITMAP_SIZE: usize = 1 << 15;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub number_of_threads: u8,
    pub thread_size: usize,
    pub save_thread_size: usize,
    pub number_of_generate_inputs: u16,
    pub number_of_deterministic_mutations: usize,
    pub max_tree_size: usize,
    pub bitmap_size: usize,
    pub path_to_bin_target: String,
    pub path_to_grammar: String,
    pub path_to_workdir: String,
    pub save_intervall: u64,
    pub save_state: bool,
    pub no_feedback_mode: bool, //When true the fuzzer only uses the generation method and no mutations
    pub dump_mode: bool, //When true the fuzzer saves every input that is tested (up to a maximum of 5000 and then cycling)
    pub arguments: Vec<String>,
}
