use crate::dwarf_data::{DwarfData, Error as DwarfError};
use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::signal::Signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::collections::HashMap;
use std::os::unix::process::CommandExt;
use std::process::Child;

struct BreakPoint {
    addr: usize,
    val: u8,
}

pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    ptrace::traceme().or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace TRACEME failed",
    )))
}
pub struct Inferior {
    child: Child,
    breakpoints: HashMap<usize, BreakPoint>,
}

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(target: &str, args: &Vec<String>) -> Option<Inferior> {
        let mut command = std::process::Command::new(target);
        command.args(args);
        unsafe {
            command.pre_exec(child_traceme);
        }
        let child = command.spawn().ok()?;
        let mut inferior = Inferior {
            child: child,
            breakpoints: HashMap::new(),
        };
        Some(inferior)
    }

    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }

    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => Status::Exited(exit_code),
            WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                Status::Stopped(signal, regs.rip as usize)
            }
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }

    pub fn continue_exec(&mut self) -> Result<Status, nix::Error> {
        // 需要完成断点恢复(如果在断点位置)
        let regs = ptrace::getregs(self.pid())?;
        let breakpoint_addr = (regs.rip - 1) as usize;
        // 不是breakpoint(用户按下了Ctrl+C)
        if !self.breakpoints.contains_key(&breakpoint_addr) {
            ptrace::cont(self.pid(), None)?;
            return Ok(self.wait(None)?);
        }
        // 恢复代码内容
        let mut regs = ptrace::getregs(self.pid())?;
        self.write_byte(breakpoint_addr, self.breakpoints[&breakpoint_addr].val)?;
        regs.rip = breakpoint_addr as u64;
        ptrace::setregs(self.pid(), regs)?;
        // 执行下一步
        ptrace::step(self.pid(), None)?;
        let status = self.wait(None)?;
        // 正常执行
        if let Status::Stopped(Signal::SIGTRAP, _) = status {
            // restore 0xcc in the breakpoint location
            self.write_byte(breakpoint_addr, 0xCC)?;
            ptrace::cont(self.pid(), None)?;
            return Ok(self.wait(None)?);
        }
        // 出错
        Ok(status)
    }

    pub fn kill(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Killing running inferior (pid {})", self.pid());
        self.child.kill()?;
        self.wait(None)?;
        Ok(())
    }
    pub fn print_backtrace(&self, debug_data: &DwarfData) -> Result<(), nix::Error> {
        let regs = ptrace::getregs(self.pid())?;
        let rip = regs.rip as usize;
        let line = debug_data.get_line_from_addr(rip);
        let function = debug_data.get_function_from_addr(rip);
        if let Some(line) = line {
            if let Some(function) = function {
                println!("{} ({}:{})", function, line.file, line.number);
            }
        }
        Ok(())
    }

    pub fn set_breakpoint(&mut self, breakpoint_list: &Vec<usize>) -> Result<(), nix::Error> {
        for breakpoint in breakpoint_list {
            let orig_byte = self.write_byte(*breakpoint, 0xCC)?;
            self.breakpoints.insert(
                *breakpoint,
                BreakPoint {
                    addr: *breakpoint,
                    val: orig_byte,
                },
            );
        }
        Ok(())
    }
}
use std::mem::size_of;

fn align_addr_to_word(addr: usize) -> usize {
    addr & (-(size_of::<usize>() as isize) as usize)
}

impl Inferior {
    fn write_byte(&mut self, addr: usize, val: u8) -> Result<u8, nix::Error> {
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        let orig_byte = (word >> 8 * byte_offset) & 0xff;
        let masked_word = word & !(0xff << 8 * byte_offset);
        let updated_word = masked_word | ((val as u64) << 8 * byte_offset);
        ptrace::write(
            self.pid(),
            aligned_addr as ptrace::AddressType,
            updated_word as *mut std::ffi::c_void,
        )?;
        Ok(orig_byte as u8)
    }
}
