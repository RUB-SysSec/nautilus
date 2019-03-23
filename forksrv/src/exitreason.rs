use libc;
use nix::sys::signal;
use nix::sys::signal::Signal;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExitReason {
    Normal(u8),
    Timeouted,
    Signaled(i32),
    Stopped(i32),
}

impl ExitReason {
    pub fn from_int(status: i32) -> ExitReason {
        if unsafe { libc::WIFSIGNALED(status) } {
            let sigi = unsafe { libc::WTERMSIG(status) };
            if Signal::from_c_int(sigi).expect("RAND_2991405959") != signal::SIGVTALRM {
                return ExitReason::Signaled(sigi);
            }
            return ExitReason::Timeouted;
        }
        if unsafe { libc::WIFSTOPPED(status) } {
            return ExitReason::Stopped(unsafe { libc::WSTOPSIG(status) });
        }
        if unsafe { libc::WIFEXITED(status) } {
            return ExitReason::Normal(unsafe { libc::WEXITSTATUS(status) } as u8);
        }
        unreachable!();
    }
}
