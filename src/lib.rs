mod reclass;
use reclass::*;

mod memflow_wrapper;
use memflow_wrapper::*;

use std::ffi::c_void;
use std::ptr;
use std::slice;

use memflow::*;
use memflow_win32::*;

#[no_mangle]
pub extern "C" fn EnumerateProcesses(callback: EnumerateProcessCallback) {
    if let Ok(mut memflow) = unsafe { lock_memflow() } {
        if let Ok(proc_list) = memflow.kernel.process_info_list() {
            for proc_info in proc_list.iter() {
                let mut proc =
                    Win32Process::with_kernel_ref(&mut memflow.kernel, proc_info.to_owned());
                if let Ok(main_module) = proc.main_module_info() {
                    let mut proc_data = EnumerateProcessData::new(
                        proc_info.pid as ProcessId,
                        &main_module.name,
                        &main_module.path,
                    );
                    (callback)(&mut proc_data)
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn EnumerateRemoteSectionsAndModules(
    handle: ProcessHandle,
    callback_section: EnumerateRemoteSectionsCallback,
    callback_module: EnumerateRemoteModulesCallback,
) {
    if let Ok(mut memflow) = unsafe { lock_memflow() } {
        let parse_sections = memflow.config.parse_sections;
        if let Some(proc) = memflow.get_process_mut(handle as u32) {
            // iterate sections
            if parse_sections {
                let mut maps = proc.virt_mem.virt_translation_map();
                maps.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

                // TODO: sections need drastic improvement
                let mut section_vaddr = 0;
                let mut section_size = 0;
                for (vaddr, size, _paddr) in maps
                    .iter()
                    .filter(|(vaddr, _, _)| vaddr.as_u64() < 0xFFFF000000000000u64)
                {
                    if section_vaddr + section_size != vaddr.as_u64() {
                        if section_size > 0 {
                            let mut section_data = EnumerateRemoteSectionData::new(
                                section_vaddr as *mut c_void,
                                section_size as usize,
                            );

                            (callback_section)(&mut section_data);
                        }

                        section_vaddr = vaddr.as_u64();
                        section_size = *size as u64;
                    } else {
                        section_size += *size as u64;
                    }
                }
            }

            // iterate modules
            if let Ok(module_list) = proc.module_list() {
                for module in module_list.iter() {
                    let mut module_data = EnumerateRemoteModuleData::new(
                        module.base.as_u64() as *mut c_void,
                        module.size as usize,
                        &module.path,
                    );
                    (callback_module)(&mut module_data);
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn OpenRemoteProcess(id: ProcessId, _desired_access: i32) -> ProcessHandle {
    if let Ok(mut memflow) = unsafe { lock_memflow() } {
        match memflow.open_process(id as u32) {
            Ok(handle) => handle as ProcessHandle,
            Err(_) => ptr::null_mut(),
        }
    } else {
        ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn IsProcessValid(handle: ProcessHandle) -> bool {
    if let Ok(mut memflow) = unsafe { lock_memflow() } {
        memflow.is_process_alive(handle as u32)
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn CloseRemoteProcess(handle: ProcessHandle) {
    if let Ok(mut memflow) = unsafe { lock_memflow() } {
        memflow.close_process(handle as u32);
    }
}

#[no_mangle]
pub extern "C" fn ReadRemoteMemory(
    handle: ProcessHandle,
    address: *mut c_void,
    buffer: *mut c_void,
    offset: i32,
    size: i32,
) -> bool {
    if let Ok(mut memflow) = unsafe { lock_memflow() } {
        if let Some(proc) = memflow.get_process_mut(handle as u32) {
            let slice = unsafe { slice::from_raw_parts_mut(buffer as *mut u8, size as usize) };
            proc.virt_mem
                .virt_read_raw_into((address as u64).wrapping_add(offset as u64).into(), slice)
                .is_ok()
        } else {
            false
        }
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn WriteRemoteMemory(
    handle: ProcessHandle,
    address: *mut c_void,
    buffer: *mut c_void,
    offset: i32,
    size: i32,
) -> bool {
    if let Ok(mut memflow) = unsafe { lock_memflow() } {
        if let Some(proc) = memflow.get_process_mut(handle as u32) {
            let slice = unsafe { slice::from_raw_parts_mut(buffer as *mut u8, size as usize) };
            proc.virt_mem
                .virt_write_raw((address as u64).wrapping_add(offset as u64).into(), slice)
                .is_ok()
        } else {
            false
        }
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn ControlRemoteProcess(_handle: ProcessHandle, _action: i32) {}

#[no_mangle]
pub extern "C" fn AttachDebuggerToProcess(_id: ProcessId) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn DetachDebuggerFromProcess(_id: ProcessId) {}

#[no_mangle]
pub extern "C" fn AwaitDebugEvent(_event: *mut c_void /*DebugEvent*/, _timeout: i32) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn HandleDebugEvent(_event: *mut c_void /*DebugEvent*/) {}

#[no_mangle]
pub extern "C" fn SetHardwareBreakpoint(
    _id: ProcessId,
    _address: *mut c_void,
    _reg: i32,
    _ty: i32,
    _size: i32,
    _set: bool,
) -> bool {
    false
}
