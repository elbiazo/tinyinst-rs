use std::{
    ffi::{c_char, CString},
    path::Path,
};
#[cxx::bridge]
pub mod common {
    // C++ types and signatures exposed to Rust.
    unsafe extern "C++" {
        include!("common.h");
        fn GetCurTime() -> u64;
    }
}

#[cxx::bridge]
pub mod litecov {
    #[repr(u32)]
    #[derive(Debug)]
    enum RunResult {
        OK,
        CRASH,
        HANG,
        OTHER_ERROR,
    }

    unsafe extern "C++" {
        // for constructors.
        include!("shim.h");
        include!("tinyinstinstrumentation.h");
        include!("aflcov.h");

        type ModuleCovData;
        pub fn ClearInstrumentationData(self: Pin<&mut ModuleCovData>);
        pub fn ClearCmpCoverageData(self: Pin<&mut ModuleCovData>);

        type Coverage;
        type ModuleCoverage;

        pub fn coverage_new() -> UniquePtr<Coverage>;

        pub unsafe fn get_coverage_map(
            bitmap: *mut u8,
            map_size: usize,
            coverage: Pin<&mut Coverage>,
        );

        // TinyinstInstrumentation
        type TinyInstInstrumentation;
        pub fn tinyinstinstrumentation_new() -> UniquePtr<TinyInstInstrumentation>;

        type RunResult;
        // type Coverage;
        pub unsafe fn Init(
            self: Pin<&mut TinyInstInstrumentation>,
            argc: i32,
            argv: *mut *mut c_char,
        );
        pub unsafe fn Run(
            self: Pin<&mut TinyInstInstrumentation>,
            argc: i32,
            argv: *mut *mut c_char,
            init_timeout: u32,
            timeout: u32,
        ) -> RunResult;

        pub unsafe fn RunWithCrashAnalysis(
            self: Pin<&mut TinyInstInstrumentation>,
            argc: i32,
            argv: *mut *mut c_char,
            init_timeout: u32,
            timeout: u32,
        ) -> RunResult;

        pub fn CleanTarget(self: Pin<&mut TinyInstInstrumentation>);
        pub fn HasNewCoverage(self: Pin<&mut TinyInstInstrumentation>) -> bool;

        pub fn GetCoverage(
            self: Pin<&mut TinyInstInstrumentation>,
            coverage: Pin<&mut Coverage>,
            afl_coverage: &mut Vec<u64>,
            clear_coverage: bool,
        );
        pub fn ClearCoverage(self: Pin<&mut TinyInstInstrumentation>);
        pub fn IgnoreCoverage(
            self: Pin<&mut TinyInstInstrumentation>,
            coverage: Pin<&mut Coverage>,
        );

        // Testing AFLCOV
        // type AFLCov;
        // pub unsafe fn aflcov_new(coverage: *mut u8, capacity: usize) -> UniquePtr<AFLCov>;
        // pub fn add_coverage(self: Pin<&mut AFLCov>, addr: u8);
    }
}

use cxx::UniquePtr;
impl litecov::TinyInstInstrumentation {
    pub fn new() -> UniquePtr<litecov::TinyInstInstrumentation> {
        litecov::tinyinstinstrumentation_new()
    }
}

impl litecov::Coverage {
    pub fn new() -> UniquePtr<litecov::Coverage> {
        litecov::coverage_new()
    }
}

pub struct TinyInst {
    tinyinst_ptr: UniquePtr<litecov::TinyInstInstrumentation>,
    program_args: Vec<String>,
    coverage_ptr: UniquePtr<litecov::Coverage>,
    timeout: u32,
}

impl TinyInst {
    pub unsafe fn new(
        tinyinst_args: Vec<String>,
        program_args: Vec<String>,
        timeout: u32,
    ) -> TinyInst {
        if !Path::new(format!("{}", program_args[0]).as_str()).exists() {
            panic!("{} does not exist", program_args[0]);
        }
        let mut tinyinst_ptr = litecov::TinyInstInstrumentation::new();

        let tinyinst_args_cstr: Vec<CString> = tinyinst_args
            .iter()
            .map(|arg| CString::new(arg.as_str()).unwrap())
            .collect();

        let mut tinyinst_args_ptr: Vec<*mut c_char> = tinyinst_args_cstr
            .iter()
            .map(|arg| arg.as_ptr() as *mut c_char)
            .collect();
        tinyinst_args_ptr.push(std::ptr::null_mut());

        // Init TinyInst with Tinyinst arguments.
        tinyinst_ptr
            .pin_mut()
            .Init(tinyinst_args.len() as i32, tinyinst_args_ptr.as_mut_ptr());

        TinyInst {
            tinyinst_ptr,
            program_args,
            timeout,
            coverage_ptr: litecov::Coverage::new(),
        }
    }

    pub unsafe fn run(&mut self) -> litecov::RunResult {
        let program_args_cstr: Vec<CString> = self
            .program_args
            .iter()
            .map(|arg| CString::new(arg.as_str()).unwrap())
            .collect();

        let mut program_args_ptr: Vec<*mut c_char> = program_args_cstr
            .iter()
            .map(|arg| arg.as_ptr() as *mut c_char)
            .collect();
        program_args_ptr.push(std::ptr::null_mut());
        self.tinyinst_ptr.pin_mut().Run(
            self.program_args.len() as i32,
            program_args_ptr.as_mut_ptr(),
            self.timeout,
            self.timeout,
        )
    }

    // pub unsafe fn bitmap_coverage(
    //     &mut self,
    //     bitmap: *mut u8,
    //     map_size: usize,
    //     clear_coverage: bool,
    // ) {
    //     self.tinyinst_ptr
    //         .pin_mut()
    //         .GetCoverage(self.coverage_ptr.pin_mut(), clear_coverage);
    //     litecov::get_coverage_map(bitmap, map_size, self.coverage_ptr.pin_mut());
    // }

    pub fn vec_coverage(&mut self, afl_coverage: &mut Vec<u64>, clear_coverage: bool) {
        // Clear coverage if there was previous coverage
        afl_coverage.clear();
        self.tinyinst_ptr.pin_mut().GetCoverage(
            self.coverage_ptr.pin_mut(),
            afl_coverage,
            clear_coverage,
        );
        // This will mark coverage we have seen as already seen coverage and won't report it again.
        self.ignore_coverage();
    }
    fn ignore_coverage(&mut self) {
        self.tinyinst_ptr
            .pin_mut()
            .IgnoreCoverage(self.coverage_ptr.pin_mut());
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{Seek, Write};
    #[test]
    fn tinyinst_ok() {
        let tinyinst_args = vec!["-instrument_module".to_string(), "test.exe".to_string()];
        // Create file to test.
        let mut file = File::create(".\\test\\test_file.txt").unwrap();
        file.write_all(b"test1").unwrap();

        let program_args = vec![
            ".\\test\\test.exe".to_string(),
            ".\\test\\test_file.txt".to_string(),
        ];
        let mut coverage = Vec::new();

        unsafe {
            let mut tinyinst = super::TinyInst::new(tinyinst_args, program_args, 5000);

            // First test case
            let result = tinyinst.run();
            tinyinst.vec_coverage(&mut coverage, true);
            assert_eq!(result, super::litecov::RunResult::OK);
            assert_eq!(coverage.len() <= 1412, true);

            // Second test case for b
            file.seek(std::io::SeekFrom::Start(0)).unwrap();
            file.write_all(b"b").unwrap();
            let result = tinyinst.run();
            tinyinst.vec_coverage(&mut coverage, true);
            assert_eq!(result, super::litecov::RunResult::OK);

            // Check if it contains address to if c == 'b' branch. Sometimes it gets address to memset function. Weird windows crap probably?
            assert_eq!(coverage.contains(&4151), true);

            // Second test case for ba
            file.seek(std::io::SeekFrom::Start(0)).unwrap();
            file.write_all(b"ba").unwrap();
            let result = tinyinst.run();
            tinyinst.vec_coverage(&mut coverage, true);
            assert_eq!(result, super::litecov::RunResult::OK);

            // Check if it contains address to if c == 'a' branch. Sometimes it gets address to memset function.
            assert_eq!(coverage.contains(&4174), true);
        }
    }
    #[test]
    fn tinyinst_crash() {
        let tinyinst_args = vec!["-instrument_module".to_string(), "test.exe".to_string()];

        let program_args = vec![
            ".\\test\\test.exe".to_string(),
            ".\\test\\crash_input.txt".to_string(),
        ];
        let mut coverage = Vec::new();

        unsafe {
            let mut tinyinst = super::TinyInst::new(tinyinst_args, program_args, 5000);
            let result = tinyinst.run();
            tinyinst.vec_coverage(&mut coverage, true);
            assert_eq!(result, super::litecov::RunResult::CRASH);
            // assert_eq!(bitmap.iter().filter(|&x| *x == 1).count(), 1307);
        }
    }
}
