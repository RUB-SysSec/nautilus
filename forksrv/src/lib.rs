extern crate libc;
extern crate nix;
#[macro_use]
extern crate quick_error;
#[macro_use]
extern crate serde_derive;
extern crate tempfile;

pub mod error;
pub mod exitreason;

use nix::fcntl::*;
use nix::sys::mman::*;
use nix::sys::signal::kill;
use nix::sys::signal::Signal::SIGCONT;
use nix::sys::signal::*;
use nix::sys::wait::WaitStatus::*;
use nix::sys::wait::*;
use nix::unistd::*;
use std::ffi::CString;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::mem;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::process;

pub use error::{descr_err, SubprocessError};
use error::{path_err, SpawnError};

#[derive(Debug)]
pub struct ForkServer<T> {
    child_pid: i32,
    shm_file: File,
    inp_file: tempfile::NamedTempFile,
    pub out_path: String,
    pub err_path: String,
    shared_data: *mut T,
}

impl<T> ForkServer<T> {
    pub fn new(
        path: &String,
        args: &Vec<String>,
        out_path: String,
        err_path: String,
    ) -> Result<Self, SubprocessError> {
        let (shm_file, shared_data) = ForkServer::<T>::create_shm()?;
        let inp_file = tempfile::NamedTempFile::new()?;
        let inp_file_path = inp_file
            .path()
            .to_str()
            .expect("temp path should be unicode!")
            .to_string();
        let child = ForkServer::<T>::start(
            path,
            args,
            &shm_file,
            &inp_file.as_file(),
            &out_path,
            &err_path,
            &inp_file_path,
        )?;
        return Ok(ForkServer {
            child_pid: child,
            shm_file,
            inp_file,
            out_path,
            err_path,
            shared_data,
        });
    }

    pub fn get_shared_mut<'a>(&'a mut self) -> &'a mut T {
        unsafe { return &mut *self.shared_data }
    }

    pub fn get_shared<'a>(&'a self) -> &'a T {
        unsafe { return &*self.shared_data }
    }

    pub fn run_on<I: AsRef<[u8]>>(&mut self, data: &I) -> Result<(), SubprocessError> {
        let mut inp_file = self.inp_file.as_file();
        inp_file.set_len(0)?;
        inp_file.seek(SeekFrom::Start(0))?;
        inp_file.write(data.as_ref())?;
        inp_file.seek(SeekFrom::Start(0))?;
        kill(self.child_pid, SIGCONT)?;
        let res = waitpid(self.child_pid, Some(WUNTRACED))
            .expect("waitpid failed - this shouldn't happen");
        match res {
            Exited(_, exitcode) => {
                return descr_err(&format!("Parent died on run with exitcode {}", exitcode))
            }
            Stopped(_, sig) if sig == SIGSTOP => return Ok(()),
            _ => return descr_err(&format!("Parent died on run {:?}", res)),
        };
    }

    //private functions

    fn create_shm() -> Result<(File, *mut T), SubprocessError> {
        let len = mem::size_of::<T>();
        let shm_file = tempfile::tempfile()?;
        shm_file.set_len(len as u64)?;
        let prot = PROT_READ | PROT_WRITE;
        let flags = MAP_SHARED;
        let ptr = mmap(
            0 as *mut nix::c_void,
            len,
            prot,
            flags,
            shm_file.as_raw_fd(),
            0,
        )?;
        return Ok((shm_file, ptr as *mut T));
    }

    fn get_filename(path: &String) -> Result<String, SpawnError> {
        let path_obj = Path::new(&path);

        if !path_obj.is_absolute() {
            return path_err("should be absolute");
        }

        let name = match path_obj.file_name() {
            Some(name) => name,
            _ => return path_err("should be a file"),
        };

        match name.to_os_string().into_string() {
            Ok(res) => return Ok(res),
            _ => return path_err("should be unicode"),
        }
    }

    fn run_process(
        path: &String,
        args: &Vec<String>,
        shm_file: &File,
        inp_file: &File,
        out_file: &String,
        err_file: &String,
        inp_file_path: &String,
    ) -> Result<(), SpawnError> {
        let filename = ForkServer::<T>::get_filename(path)?;
        let cpath = CString::new(path.clone())?;
        let args_iter = args
            .iter()
            .map(|arg| if arg == "@@" { inp_file_path } else { arg });
        let args_iter = Some(&filename).into_iter().chain(args_iter); //add filename as argv[0]
        let cargs = args_iter
            .map(|s| Ok(CString::new(s.clone())?))
            .collect::<Result<Vec<CString>, SpawnError>>()?; //convert all String args to CStrin args
        let shm_fd = shm_file.as_raw_fd();
        let inp_fd = inp_file.as_raw_fd();
        let env = vec![
            CString::new("LD_BIND_NOW=1").expect("RAND_508190816"),
            CString::new(format!("ROFL_SHM_FD={}", shm_fd)).expect("RAND_3630438482"),
            CString::new(format!("ROFL_INP_FD={}", inp_fd)).expect("RAND_734314699"),
            CString::new(format!("ROFL_OUT_PATH={}", out_file)).expect("RAND_2015012392"),
            CString::new(format!("ROFL_ERR_PATH={}", err_file)).expect("RAND_3568988286"),
            CString::new("ASAN_OPTIONS=exitcode=223,abort_on_erro=true").expect("RAND_2089158993"),
        ];
        fcntl(
            shm_fd,
            F_SETFD(FdFlag::from_bits(0).expect("RAND_22127389")),
        )?; //unset O_CLOEXEC flags such that the child can access the fds
        fcntl(
            inp_fd,
            F_SETFD(FdFlag::from_bits(0).expect("RAND_1556107492")),
        )?;
        //use inp_fd instead of the original stdin
        dup2(inp_fd, 0)?;
        execve(&cpath, &cargs, &env)?;
        unreachable!()
    }

    fn start(
        path: &String,
        args: &Vec<String>,
        shm_file: &File,
        inp_file: &File,
        out_file: &String,
        err_file: &String,
        inp_file_path: &String,
    ) -> Result<(i32), SubprocessError> {
        match fork().expect("fork failed") {
            ForkResult::Parent { child } => {
                let res = waitpid(child, Some(WUNTRACED))
                    .expect("waitpid failed - this shouldn't happen");
                match res {
                    Exited(_, exitcode) => {
                        return descr_err(&format!(
                            "child died prematurely with exitcode {}",
                            exitcode
                        ))
                    }
                    Stopped(_, sig) if sig == SIGSTOP => return Ok(child),
                    Signaled(_, sig, _) if sig == SIGSEGV => {
                        return descr_err(&format!(
                            "child signaled {:?} prematurely, broken instrumentation?",
                            sig
                        ))
                    }
                    _ => return descr_err(&format!("Unexpected wait result {:?}", res)),
                };
            }
            ForkResult::Child => {
                {
                    let res = ForkServer::<T>::run_process(
                        path,
                        args,
                        shm_file,
                        inp_file,
                        out_file,
                        err_file,
                        inp_file_path,
                    );
                    let err = res.err();
                    print!("Executing Target failed {:?}\n", err)
                }
                process::exit(0x0f00);
            }
        }
    }
}
