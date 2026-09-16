//! Start the converter suspended, attach its job, then resume it. Closing the
//! job also kills the converter if Explorer terminates the thumbnail worker.
use anyhow::{ensure, Context, Result};
use std::{ffi::OsString, os::windows::ffi::OsStrExt, path::Path};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::{
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
                SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
            },
            Threading::{
                CreateProcessW, GetExitCodeProcess, ResumeThread, TerminateProcess,
                WaitForSingleObject, CREATE_NO_WINDOW, CREATE_SUSPENDED, PROCESS_INFORMATION,
                STARTUPINFOW,
            },
        },
    },
};

struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

pub struct Process {
    process: Handle,
    job: Handle,
}
impl Process {
    pub fn spawn(
        executable: &Path,
        args: &[OsString],
        cwd: &Path,
        memory_limit: usize,
    ) -> Result<Self> {
        let mut command = quoted(executable.as_os_str().encode_wide())?;
        for arg in args {
            command.push(32);
            command.extend(quoted(arg.encode_wide())?);
        }
        command.push(0);
        let app: Vec<_> = executable
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let directory: Vec<_> = cwd.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {
            let job = Handle(CreateJobObjectW(None, PCWSTR::null())?);
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags =
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_PROCESS_MEMORY;
            limits.ProcessMemoryLimit = memory_limit;
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                std::mem::size_of_val(&limits) as u32,
            )?;
            let startup = STARTUPINFOW {
                cb: std::mem::size_of::<STARTUPINFOW>() as u32,
                ..Default::default()
            };
            let mut info = PROCESS_INFORMATION::default();
            CreateProcessW(
                PCWSTR(app.as_ptr()),
                PWSTR(command.as_mut_ptr()),
                None,
                None,
                false,
                CREATE_NO_WINDOW | CREATE_SUSPENDED,
                None,
                PCWSTR(directory.as_ptr()),
                &startup,
                &mut info,
            )
            .context("could not start render process")?;
            let thread = Handle(info.hThread);
            let result = Self {
                process: Handle(info.hProcess),
                job,
            };
            AssignProcessToJobObject(result.job.0, result.process.0)
                .context("could not isolate render process")?;
            ensure!(
                ResumeThread(thread.0) != u32::MAX,
                "could not resume render process"
            );
            Ok(result)
        }
    }

    pub fn poll(&mut self) -> Result<Option<u32>> {
        unsafe {
            match WaitForSingleObject(self.process.0, 0) {
                WAIT_OBJECT_0 => {
                    let mut code = 0;
                    GetExitCodeProcess(self.process.0, &mut code)?;
                    Ok(Some(code))
                }
                WAIT_TIMEOUT => Ok(None),
                _ => Err(std::io::Error::last_os_error().into()),
            }
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            let _ = TerminateJobObject(self.job.0, 1);
            // Also cover failure to attach the suspended child to its job.
            let _ = TerminateProcess(self.process.0, 1);
            WaitForSingleObject(self.process.0, 5000);
        }
    }
}

// Windows CRT command-line escaping, including quotes and trailing backslashes.
fn quoted(input: impl Iterator<Item = u16>) -> Result<Vec<u16>> {
    let mut out = vec![34];
    let mut slashes = 0;
    for ch in input {
        ensure!(ch != 0, "NUL in converter argument");
        if ch == 92 {
            slashes += 1;
            continue;
        }
        out.extend(std::iter::repeat_n(
            92,
            if ch == 34 { slashes * 2 + 1 } else { slashes },
        ));
        out.push(ch);
        slashes = 0;
    }
    out.extend(std::iter::repeat_n(92, slashes * 2));
    out.push(34);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quotes_paths_without_shell_expansion() {
        for (input, expected) in [
            ("simple", "\"simple\""),
            ("a b\\", "\"a b\\\\\""),
            ("a\"b", "\"a\\\"b\""),
            ("$(whoami)&x", "\"$(whoami)&x\""),
        ] {
            assert_eq!(
                String::from_utf16(&quoted(input.encode_utf16()).unwrap()).unwrap(),
                expected
            );
        }
        assert!(quoted("bad\0arg".encode_utf16()).is_err());
    }
}
