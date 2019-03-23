extern crate forksrv;
extern crate nix;
extern crate rand;
extern crate regex;
extern crate subprocess;
extern crate time;

#[macro_use]
extern crate lazy_static;

use forksrv::{descr_err, ForkServer, SubprocessError};
use nix::sys::signal::Signal;
use rand::Rng;
use regex::Regex;
use std::env;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;

use std::collections::{HashMap, HashSet};

const LIB_EXTRACTOR: &str = "
class Feedback
  def self.test_arity(obj, method, arity)
    begin
      obj.send(method, *([nil]*arity))
    rescue ArgumentError => e
      return false if e.message.include?('wrong number of arguments')
      return true
    rescue Exception => e
      return true
    end
    return true
  end
  def self.fake_arity(obj, method)
    arity = (0..9).to_a.index{|i| test_arity(obj, method, i)}
    return 10 unless arity
    arity = -arity-1 if test_arity(obj,method,arity+1)
    return arity
  end
end
puts \"__VALUE__: #{$_current.inspect}\"
puts \"__TYPE__: #{$_current.class.inspect}\"
$_current.methods.each{|n| puts \"__FUNCTION__: #{n} #{Feedback.fake_arity($_current, n)}\" }
";

#[derive(Debug)]
struct Method {
    name: String,
    arity: i32,
}

impl Method {
    fn new(name: String, arity: i32) -> Self {
        return Method { name, arity };
    }
    fn arity(&self) -> usize {
        if self.arity >= 0 {
            return self.arity as usize;
        }
        return (((-self.arity) - 1) as usize) + (rand::thread_rng().gen::<usize>() % 3);
    }
}

#[derive(Debug)]
struct FuzzObject {
    methods: Option<Vec<Method>>,
    id: usize,
    recv: usize,
    typedesc: String,
    valdesc: String,
    rhs: String,
    deps: Vec<usize>,
}

impl FuzzObject {
    fn new(recv: usize, rhs: String, deps: Vec<usize>) -> Self {
        return FuzzObject {
            methods: None,
            recv,
            id: 0,
            rhs,
            deps,
            typedesc: "".into(),
            valdesc: "".into(),
        };
    }

    fn name(&self) -> String {
        return format!("$var_{}", self.id);
    }

    fn code(&self) -> String {
        return format!("$var_{} = {}", self.id, self.rhs);
    }
}

#[derive(Debug)]
struct ObjectSpace {
    objects: Vec<FuzzObject>,
    seen_values: HashSet<String>,
    seen_types: HashMap<String, usize>,
    obj_ids_to_calls: HashMap<usize, Vec<FuzzObject>>,
}

impl ObjectSpace {
    pub fn new() -> Self {
        return ObjectSpace {
            objects: vec![],
            seen_values: HashSet::new(),
            seen_types: HashMap::new(),
            obj_ids_to_calls: HashMap::new(),
        };
    }

    pub fn is_new(&self, obj: &FuzzObject) -> bool {
        (!self.seen_values.contains(&obj.valdesc) && (obj.valdesc.len() <= 4)
            || self.seen_types.get(&obj.typedesc).unwrap_or(&0) < &20)
    }

    fn gen_rhs(&self, recv: &FuzzObject) -> (String, Vec<usize>) {
        let mut rng = rand::thread_rng();
        let m = recv.methods.as_ref().expect("RAND_2157516637");
        let method = rng.choose(&m).expect("RAND_1450294468");
        let mut deps = (0..method.arity())
            .map(|_| rng.gen::<usize>() % self.objects.len())
            .collect::<Vec<_>>();
        let args = deps.iter().fold(String::new(), |acc, &i| {
            format!("{}{},", acc, self.objects[i].name())
        });
        deps.push(recv.id);
        return (
            format!("{}.{}({}) rescue nil", recv.name(), method.name, args),
            deps,
        );
    }

    fn gen_use_pattern(&self, recv: usize) -> (String, Vec<usize>) {
        let mut rng = rand::thread_rng();
        let mut res_code = String::new();
        let mut res_dep = vec![];
        let methods = self.obj_ids_to_calls.get(&recv).expect("RAND_236885980");
        assert!(methods.len() > 0);
        for _ in 0..50 {
            let obj = rng.choose(&methods).expect("RAND_3213692662");
            res_code += "\n";
            res_code += &obj.rhs;
            res_dep.extend(&obj.deps);
        }
        return (res_code, res_dep);
    }

    pub fn insert(&mut self, mut obj: FuzzObject) {
        assert!(obj.methods.is_some());
        self.seen_values.insert(obj.valdesc.clone());
        let count = self.seen_types.entry(obj.typedesc.clone()).or_insert(0);
        *count += 1;
        obj.id = self.objects.len();
        self.objects.push(obj);
    }

    pub fn insert_method(&mut self, obj: FuzzObject) {
        let calls = self.obj_ids_to_calls.entry(obj.recv).or_insert(vec![]);
        (*calls).push(obj);
    }

    pub fn next_id(&self) -> usize {
        return self.objects.len();
    }

    fn get_deps_recursive(&self, deps: &Vec<usize>) -> Vec<usize> {
        let mut dep_set = HashSet::new();
        let mut deps = deps.clone();
        while deps.len() > 0 {
            let dep = deps.pop().expect("RAND_3083428526");
            if !dep_set.contains(&dep) {
                dep_set.insert(dep);
                let mut others = self.objects[dep].deps.clone();
                deps.append(&mut others);
            }
        }
        let mut deps = dep_set.into_iter().collect::<Vec<_>>();
        deps.sort();
        return deps;
    }

    pub fn gen_exploitation_script(&self) -> Option<String> {
        if self.obj_ids_to_calls.len() == 0 {
            return None;
        }
        let mut rng = rand::thread_rng();
        let index = rng.gen::<usize>() % self.obj_ids_to_calls.len();
        let recv = self
            .obj_ids_to_calls
            .keys()
            .skip(index)
            .next()
            .expect("RAND_2050974741");
        let (rhs, new_deps) = self.gen_use_pattern(recv.clone());
        let deps = self.get_deps_recursive(&new_deps);
        let script = deps.iter().fold(String::new(), |acc, &i| {
            format!("{}\n{}", acc, self.objects[i].code())
        });
        return Some(format!("{}\n {}", script, rhs));
    }

    pub fn gen_exploration_script(&self) -> (String, FuzzObject) {
        let mut rng = rand::thread_rng();
        let recv = rng.choose(&self.objects).expect("RAND_3981508845");
        let (rhs, new_deps) = self.gen_rhs(recv);
        let deps = self.get_deps_recursive(&new_deps);
        let script = deps.iter().fold(String::new(), |acc, &i| {
            format!("{}\n{}", acc, self.objects[i].code())
        });
        return (
            format!("{}\n$_current = {}", script, rhs),
            FuzzObject::new(recv.id, rhs, new_deps),
        );
    }
}

#[repr(C)]
struct FeedbackData {
    run_bitmap: [u8; 1 << 15],
    magic: u64,
    status: i32,
}

impl fmt::Debug for FeedbackData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Feedback {{ run_bitmap [...] magic: {}, status: {} }}",
            self.magic, self.status
        )
    }
}

struct Fuzzer {
    forksrv: ForkServer<FeedbackData>,
    infosrv: ForkServer<FeedbackData>,
    bitmap: [u8; 1 << 15],
    objs: ObjectSpace,
    pub target_path: String,
    pub target_args: Vec<String>,
}

impl Fuzzer {
    pub fn new(path: String, args: Vec<String>) -> Result<Self, SubprocessError> {
        let fs =
            ForkServer::<FeedbackData>::new(&path, &args, "/dev/null".into(), "/dev/null".into())?;
        let is =
            ForkServer::<FeedbackData>::new(&path, &args, "/tmp/out".into(), "/tmp/err".into())?;
        let objs = ObjectSpace::new();
        return Ok(Fuzzer {
            forksrv: fs,
            infosrv: is,
            bitmap: [0; 1 << 15],
            objs,
            target_path: path,
            target_args: args,
        });
    }

    pub fn run_on(&mut self, input: &[u8]) -> Result<(), SubprocessError> {
        self.forksrv.get_shared_mut().magic = 0x1337133713371337;
        self.forksrv.run_on(&input)?;
        if self.forksrv.get_shared().magic != 0x5a5a55464c464f52 {
            return descr_err("Failed to get magic value from subprocess");
        }
        return Ok(());
    }

    pub fn term_signal_i8(&self) -> Option<i8> {
        let status = self.forksrv.get_shared().status;
        //WIFSIGNALED
        if ((((status & 0x7f) + 1) as i8) >> 1) > 0 {
            return Some((status & 0x7f) as i8);
        }
        return None;
    }

    pub fn term_signal(&self) -> Option<Signal> {
        self.term_signal_i8()
            .map(|i| Signal::from_c_int(i as i32).expect("RAND_922996219"))
    }

    pub fn has_new_bit(&mut self) -> bool {
        let mut res = false;
        let run_bitmap = self.forksrv.get_shared().run_bitmap;
        for (i, elem) in self.bitmap.iter_mut().enumerate() {
            if (*elem | run_bitmap[i]) != *elem {
                *elem |= run_bitmap[i];
                res = true;
            }
        }
        return res;
    }

    fn capture_output_on<T: AsRef<[u8]>>(&mut self, data: &T) -> Result<String, SubprocessError> {
        self.infosrv.run_on(&data)?;
        let mut file = File::open(&self.infosrv.out_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        return Ok(contents);
    }

    fn output_to_methods(&self, output: &String) -> Vec<Method> {
        lazy_static! {
            static ref M_RE: Regex =
                Regex::new(r"(?m)^__FUNCTION__: (.*) (.*)$").expect("RAND_178256966");
        }
        let matches = M_RE.captures_iter(output);
        let methods =
            matches.map(|cap| Method::new(cap[1].into(), cap[2].parse::<i32>().unwrap_or(-1)));
        let res = methods.collect::<Vec<_>>();
        return res;
    }

    fn output_to_desc(&self, output: &String) -> (Option<String>, Option<String>) {
        lazy_static! {
            static ref T_RE: Regex = Regex::new(r"(?m)^__TYPE__: (.*)$").expect("RAND_1258185188");
            static ref V_RE: Regex = Regex::new(r"(?m)^__VALUE__: (.*)$").expect("RAND_2357435597");
        }
        let mut matches = T_RE.captures_iter(output);
        let type_desc = matches.nth(0).map(|v| v[1].into());

        let mut matches = V_RE.captures_iter(output);
        let val_desc = matches.nth(0).map(|v| v[1].into());
        return (type_desc, val_desc);
    }

    //pub fn get_methods_for_current(&mut self, code: &String) -> Result<(Option<String>, Option<String>,Vec<Method>),SubprocessError> {
    //    return Ok(type_desc, val_desc, methods);
    //}

    pub fn set_obj_info(
        &mut self,
        obj: &mut FuzzObject,
        code: &String,
    ) -> Result<(), SubprocessError> {
        let data = format!("{}{}", code, LIB_EXTRACTOR);
        let resp = self.capture_output_on(&data)?;
        let methods = self.output_to_methods(&resp);
        let (type_desc, val_desc) = self.output_to_desc(&resp);
        obj.methods = Some(methods);
        obj.valdesc = val_desc.unwrap_or("".into());
        obj.typedesc = type_desc.unwrap_or("".into());
        return Ok(());
    }

    pub fn add_initial_value(&mut self, code: String) {
        let mut obj = FuzzObject::new(0, code.clone(), vec![]);
        self.set_obj_info(&mut obj, &format!("$_current = {}\n", code))
            .expect("RAND_2752912672");
        self.objs.insert(obj);
    }
}

pub fn main() {
    let pargs: Vec<String> = env::args().collect();

    let default_path = "/home/me/proggen/rust/forksrv/instrument/mruby/mruby/bin/mruby".into();
    let prog = pargs.get(1).unwrap_or(&default_path);
    let args = vec![];

    let mut fuzzer = Fuzzer::new(prog.clone(), args).expect("RAND_3059977769");

    fuzzer.add_initial_value("[]".into());
    fuzzer.add_initial_value("{}".into());
    fuzzer.add_initial_value("1".into());
    fuzzer.add_initial_value("'\\0'".into());
    fuzzer
        .add_initial_value("'longstring_with_some_more_content_to_blow_past_optimizations'".into());

    let mut count = 0;
    let start = time::now();
    loop {
        count += 1;

        //explore
        let (code, mut obj) = fuzzer.objs.gen_exploration_script();
        fuzzer
            .run_on(code.clone().as_bytes())
            .expect("RAND_1642205779");
        let sig = fuzzer.term_signal_i8();
        if let Some(sigi) = sig {
            let sigv = fuzzer.term_signal().expect("RAND_2407706175");
            //if sigv != signal::SIGVTALRM {
            print!("found signaling input: {:?} {}\n", sigv, sigi);
            let mut file =
                File::create(format!("outputs/{}_{}", sigi, count % 1000)).expect("RAND_208563844");
            file.write_all(code.as_bytes()).expect("RAND_2111594943");
        //}
        } else if fuzzer.has_new_bit() {
            if fuzzer.set_obj_info(&mut obj, &code).is_ok() {
                let mut ok = false;
                if let Some(ref methods) = obj.methods {
                    ok = methods.len() > 0;
                }
                if ok && fuzzer.objs.is_new(&obj) {
                    print!(
                        "new_obj $var_{} = {:?} # {} ({}) (iter {}, after {})\n",
                        fuzzer.objs.next_id(),
                        &obj.rhs,
                        &obj.typedesc,
                        &obj.valdesc,
                        count,
                        time::now() - start
                    );
                    fuzzer.objs.insert(obj);
                } else {
                    print!(
                        "new_call {:?} # (iter {}, after {})\n",
                        &obj.rhs,
                        count,
                        time::now() - start
                    );
                    fuzzer.objs.insert_method(obj);
                }
            }
        }

        //exploit
        if let Some(code) = fuzzer.objs.gen_exploitation_script() {
            //print!(" got expoitation: {}", code);
            fuzzer
                .run_on(code.clone().as_bytes())
                .expect("RAND_2722321624");
            let sig = fuzzer.term_signal_i8();
            if let Some(sigi) = sig {
                let sigv = fuzzer.term_signal().expect("RAND_1737335241");
                print!("found signaling input: {:?} {}\n", sigv, sigi);
                let mut file = File::create(format!("outputs/{}_{}", sigi, count % 1000))
                    .expect("RAND_3879885943");
                file.write_all(code.as_bytes()).expect("RAND_3027968306");
            }
        }
    }
}
