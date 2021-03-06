//! `wrappers` is a "thin as possible" wrapper around windows-rs. It aims to be
//! fast, while trying to remain as safe as possible, and being easy to use.
//!
//! Not all functions are designated as safe as without adding a significant
//! amount of boilerplate will always be up to the caller to make sure UB can't
//! happen. As time goes on we'll try to make as little functions unsafe.
use std::{os::raw::c_void, ptr::null_mut};

use windows::Win32::{
    Foundation::{CloseHandle, GetLastError, HANDLE, HINSTANCE},
    Security::SECURITY_ATTRIBUTES,
    System::{
        Console::{AllocConsole, FreeConsole},
        Diagnostics::{
            Debug::{ReadProcessMemory, WriteProcessMemory},
            ToolHelp::{
                CreateToolhelp32Snapshot, Module32First, Module32Next, Process32First, Process32Next,
                CREATE_TOOLHELP_SNAPSHOT_FLAGS, MODULEENTRY32, PROCESSENTRY32,
            },
        },
        LibraryLoader::{DisableThreadLibraryCalls, FreeLibraryAndExitThread, GetModuleHandleA, GetProcAddress},
        Memory::{
            VirtualAllocEx, VirtualFreeEx, VirtualProtect, VirtualProtectEx, VirtualQueryEx, MEMORY_BASIC_INFORMATION,
            PAGE_PROTECTION_FLAGS, VIRTUAL_ALLOCATION_TYPE, VIRTUAL_FREE_TYPE,
        },
        Threading::{
            CreateRemoteThread, CreateThread, GetCurrentProcess, GetProcessId, OpenProcess, WaitForSingleObject,
            LPTHREAD_START_ROUTINE, PROCESS_ACCESS_RIGHTS, THREAD_CREATION_FLAGS,
        },
    },
    UI::Input::KeyboardAndMouse::GetAsyncKeyState,
};

use crate::error::Error;

/// `size_t` is a usize which will be 4 bytes for x86 and 8 bytes for x64
#[allow(non_camel_case_types)]
pub type size_t = usize;

// Windows Data Types

/// `DWORD` is a double word, a word is 16-bits, the size is identical to the
/// size of a u32.
pub type DWORD = u32;
#[allow(non_camel_case_types)]
/// `DWORD_PTR` is a pointer as a usize so it will be 4 bytes for x86 and 8
/// bytes for x64
pub type DWORD_PTR = usize;
/// `LPVOID` is a pointer to any type.
pub type LPVOID = *mut c_void;
/// `LPCVOID` is a pointer to a constant of any type.
pub type LPCVOID = *const c_void;
/// `WCHAR` is a 16-bit unicode character.
pub type WCHAR = u16;
/// `LPCWSTR` is a long pointer to a constant wide string.
pub type LPCWSTR = WCHAR;

/// `HandleInstance` is a handle to a module/instance. This handle is used for a
/// lot of functions as they are used to identify a program that is loaded into
/// memory.
pub type HandleInstance = HINSTANCE;
/// `Handle` is a handle to an object, and not specifically a program.
pub type Handle = HANDLE;
/// `HModule` is the base address of a DLL.
pub type HModule = isize;
/// Contains information about a range of pages in the virtual address space of
/// a process.
pub type MemoryBasicInformation = MEMORY_BASIC_INFORMATION;
/// `PageProtectionFlags` is a variable that contains memory protection
/// constants.
pub type PageProtectionFlags = PAGE_PROTECTION_FLAGS;

// TODO: DOCS cc: steele
pub type VirtualFreeType = VIRTUAL_FREE_TYPE;
/// A type of memory allocation which could be a reserve, commit or change to a
/// region in the virtual memory.
pub type VirtualAllocationType = VIRTUAL_ALLOCATION_TYPE;
/// `SecurityAttributes` of a thread it will determine whether the return
/// `Handle` can be inherited by the child processes. If this is null it will
/// get a default by the system.
pub type SecurityAttributes = SECURITY_ATTRIBUTES;
/// A pointer to a function that will serve as the starting address for a
/// thread.
pub type LPThreadStartRoutine = LPTHREAD_START_ROUTINE;
/// Flags that control the creation of a thread.
pub type ThreadCreationFlags = THREAD_CREATION_FLAGS;
/// Access rights that the system will give you to the process, this is meant to
/// be used with the `open_process` function which will open the process with
/// the provided access rights.
pub type ProcessAccessRights = PROCESS_ACCESS_RIGHTS;
/// `ProcessEntry32` is an entry from a list of the processes in the system
/// address space when the snapshot from `create_tool_help32_snapshot` was
/// taken.
pub type ProcessEntry32 = PROCESSENTRY32;
/// `CreateToolhelpSnapshotFlags` are flags to indicate which parts of the
/// system should be included in the snapshot for example you would use the flag
/// `TH32CS_SNAPMODULE` to include the modules of the process.
pub type CreateToolhelpSnapshotFlags = CREATE_TOOLHELP_SNAPSHOT_FLAGS;
/// `ModuleEntry32` is used for crawling the modules of a process in most cases
/// you will be just default its value dwSize because not initializing dwSize
/// will make `module32_first` fail.
pub type ModuleEntry32 = MODULEENTRY32;

/// `get_module_handle` will get the handle of a module.
///
/// # Errors
/// If the `hInstance` returned is NULL a `Error::Handle` is returned.
pub fn get_module_handle(module_name: &str) -> Result<HandleInstance, Error> {
    let hinstance = unsafe { GetModuleHandleA(module_name) };

    if hinstance.is_negative() {
        Err(Error::Handle(unsafe { GetLastError() }))
    } else {
        Ok(hinstance)
    }
}

/// Retrieves information about a range of pages within the virtual address
/// space of a specified process.
///
/// # Errors
/// `Error::MemoryError` is returned if the function fails.
pub fn virtual_query_ex(
    process: Handle,
    address: LPCVOID,
    buffer: *mut MemoryBasicInformation,
    length: usize,
) -> Result<usize, Error> {
    let num_bytes = unsafe { VirtualQueryEx(process, address, buffer, length) };
    if num_bytes == 0 {
        Err(Error::MemoryError(unsafe { GetLastError() }))
    } else {
        Ok(num_bytes)
    }
}

/// Determines whether a key is up or down at the time the function is called,
/// and whether the key was pressed after a previous call to
/// `get_async_key_state`.
///
/// # Errors
/// If the function succeeds, the return value specifies whether the key was
/// pressed since the last call to `get_async_key_state`, and whether the key is
/// currently up or down. If the most significant bit is set, the key is down,
/// and if the least significant bit is set, the key was pressed after the
/// previous call to `get_async_key_state`. However, you should not rely on this
/// last behavior.
#[must_use]
pub fn get_async_key_state(key: i32) -> i16 { unsafe { GetAsyncKeyState(key) } }

/// Changes the protection on a region of committed pages in the virtual address
/// space of a specified process.
///
/// # Errors
/// On error `Error::Allocation` is returned.
pub fn virtual_protect_ex(
    process: Handle,
    address: LPVOID,
    size: usize,
    new_protect: PageProtectionFlags,
    old_protect: *mut PageProtectionFlags,
) -> Result<(), Error> {
    let res = unsafe { VirtualProtectEx(process, address, size, new_protect, old_protect) };

    if res.as_bool() {
        Ok(())
    } else {
        Err(Error::Allocation(unsafe { GetLastError() }))
    }
}

/// Changes the protection on a region of committed pages in the virtual address
/// space of the calling process.
///
/// # Errors
/// On error `Error::MemoryError` is returned.
pub fn virtual_protect(
    address: LPVOID,
    size: usize,
    new_protect: PageProtectionFlags,
    old_protect: *mut PageProtectionFlags,
) -> Result<(), Error> {
    let res = unsafe { VirtualProtect(address, size, new_protect, old_protect) };

    if res.as_bool() {
        Ok(())
    } else {
        Err(Error::MemoryError(unsafe { GetLastError() }))
    }
}

/// Waits until the specified object is in the signaled state or the time-out
/// interval elapses.
///
/// To enter an alertable wait state, use the `WaitForSingleObjectEx` function.
/// To wait for multiple objects, use `WaitForMultipleObjects`.
///
/// # Errors
/// If the function fails, `Error::Timeout` is returned.
pub fn wait_for_single_object(handle: Handle, milliseconds: u32) -> Result<u32, Error> {
    let res = unsafe { WaitForSingleObject(handle, milliseconds) };

    if res == 0xFFFF_FFFF {
        Err(Error::Timeout)
    } else {
        Ok(res)
    }
}

/// Creates a thread that runs in the virtual address space of another process.
///
/// Use the `CreateRemoteThreadEx` function to create a thread that runs in the
/// virtual address space of another process and optionally specify extended
/// attributes.
///
/// Providing none to `thread_attributes`, `thread_id`, or `parameter` will let
/// us default the value with a null pointer.
///
/// # Errors
/// If the function fails, `Error::ProcessNotFound` is returned.
pub fn create_remote_thread(
    process: Handle,
    thread_attributes: Option<*mut SecurityAttributes>,
    stack_size: usize,
    start_address: LPThreadStartRoutine,
    parameter: Option<LPVOID>,
    creation_flags: u32,
    thread_id: Option<*mut u32>,
) -> Result<Handle, Error> {
    let handle = unsafe {
        CreateRemoteThread(
            process,
            thread_attributes.unwrap_or(null_mut()),
            stack_size,
            start_address,
            parameter.unwrap_or(null_mut()),
            creation_flags,
            thread_id.unwrap_or(null_mut()),
        )
    };

    if handle.is_invalid() {
        Err(Error::ProcessError(unsafe { GetLastError() }))
    } else {
        Ok(handle)
    }
}

/// Creates a thread to execute within the virtual address space of the calling
/// process.
///
/// To create a thread that runs in the virtual address space of another
/// process, use the `create_remote_thread` function.
///
/// Providing none to `thread_attributes`, `thread_id`, or `parameter` will let
///  us default the value with a null pointer.
///
/// # Errors
/// If the function fails, `Error::ProcessNotFound` is returned.
pub fn create_thread(
    thread_attributes: Option<*mut SecurityAttributes>,
    stack_size: usize,
    start_address: LPThreadStartRoutine,
    parameter: Option<LPVOID>,
    creation_flags: ThreadCreationFlags,
    thread_id: Option<*mut u32>,
) -> Result<Handle, Error> {
    let res = unsafe {
        CreateThread(
            thread_attributes.unwrap_or(null_mut()),
            stack_size,
            start_address,
            parameter.unwrap_or(null_mut()),
            creation_flags,
            thread_id.unwrap_or(null_mut()),
        )
    };

    if res.is_invalid() {
        Err(Error::ProcessError(unsafe { GetLastError() }))
    } else {
        Ok(res)
    }
}

/// Closes an open object handle.
///
/// # Errors
/// If the function fails, `Error::Handle` is returned.
pub fn close_handle(handle: Handle) -> Result<(), Error> {
    let res = unsafe { CloseHandle(handle) };

    if res.as_bool() {
        Ok(())
    } else {
        Err(Error::Handle(unsafe { GetLastError() }))
    }
}

/// Retrieves a pseudo handle for the current process.
#[must_use]
pub fn get_current_process() -> Handle { unsafe { GetCurrentProcess() } }

/// Allocates a console for the calling process. A process is only able to have
/// one console, this function will fail if it already has a console. If you
/// want to get rid of the existing console you should call our `free_console`
/// function.
/// # Errors
pub fn alloc_console() -> Result<(), Error> {
    let success = unsafe { AllocConsole() };

    if success.as_bool() {
        Ok(())
    } else {
        Err(Error::ConsoleAllocation(unsafe { GetLastError() }))
    }
}

/// Frees a console from the calling process.
/// # Errors
pub fn free_console() -> Result<(), Error> {
    let success = unsafe { FreeConsole() };

    if success.as_bool() {
        Ok(())
    } else {
        Err(Error::ConsoleDeallocation(unsafe { GetLastError() }))
    }
}

/// Firstly `FreeLibrary` is called which frees the DLL and if needed decrements
/// the reference count, when the reference count reaches zero the module will
/// be unloaded from the address space and the handle will no longer be valid
/// then `ExitThread` will be called to terminate the calling thread.
pub fn free_library_and_exit_thread(module_handle: HandleInstance, exit_code: DWORD) {
    unsafe {
        FreeLibraryAndExitThread(module_handle, exit_code);
    }
}

/// Opens an existing local process object.
#[must_use]
pub fn open_process(desired_access: ProcessAccessRights, inherit_handle: bool, process_id: DWORD) -> Handle {
    unsafe { OpenProcess(desired_access, inherit_handle, process_id) }
}

/// Takes a snapshot of the specified processes, as well as the heaps, modules,
/// and threads used by these processes.
///
/// # Errors
/// If the function fails, `Error::MemoryError` is returned.
pub fn create_tool_help32_snapshot(flags: CreateToolhelpSnapshotFlags, process_id: DWORD) -> Result<Handle, Error> {
    let res = unsafe { CreateToolhelp32Snapshot(flags, process_id) };
    if res.is_invalid() {
        Err(Error::MemoryError(unsafe { GetLastError() }))
    } else {
        Ok(res)
    }
}

/// Retrieves information about the first module associated with a process.
///
/// # Errors
/// If the function fails, `Error::MemoryError` is returned.
pub fn module32_first(snapshot: Handle, module_entry: &mut ModuleEntry32) -> Result<(), Error> {
    let res = unsafe { Module32First(snapshot, module_entry) };
    if res.as_bool() {
        Ok(())
    } else {
        Err(Error::MemoryError(unsafe { GetLastError() }))
    }
}

/// Retrieves information about the next module associated with a process or
/// thread.
///
/// # Errors
/// If the function fails, `Error::MemoryError` is returned.
pub fn module32_next(snapshot: Handle, module_entry: &mut ModuleEntry32) -> Result<(), Error> {
    let res = unsafe { Module32Next(snapshot, module_entry) };
    if res.as_bool() {
        Ok(())
    } else {
        Err(Error::MemoryError(unsafe { GetLastError() }))
    }
}

/// Retrieves information about the first process encountered in a system
/// snapshot.
///
/// # Errors
/// If the function fails, `Error::MemoryError` is returned.
pub fn process32_first(snapshot: Handle, process_entry: &mut ProcessEntry32) -> Result<(), Error> {
    let res = unsafe { Process32First(snapshot, process_entry) };
    if res.as_bool() {
        Ok(())
    } else {
        Err(Error::MemoryError(unsafe { GetLastError() }))
    }
}

/// Retrieves information about the next process recorded in a system snapshot.
///
/// # Errors
/// If the function fails, `Error::MemoryError` is returned.
pub fn process32_next(snapshot: Handle, process_entry: &mut ProcessEntry32) -> Result<(), Error> {
    let res = unsafe { Process32Next(snapshot, process_entry) };
    if res.as_bool() {
        Ok(())
    } else {
        Err(Error::MemoryError(unsafe { GetLastError() }))
    }
}

/// Writes data to an area of memory in a specified process. The entire area to
/// be written to must be accessible or the operation fails.
///
/// # Errors
/// If the function fails, `Error::MemoryError` is returned.
pub fn write_process_memory(
    process_handle: Handle,
    base_address: LPVOID,
    buffer: LPCVOID,
    size: size_t,
    number_of_bytes_written: Option<*mut size_t>,
) -> Result<(), Error> {
    let result = unsafe {
        WriteProcessMemory(
            process_handle,
            base_address,
            buffer,
            size,
            number_of_bytes_written.unwrap_or(null_mut()),
        )
    };
    if result.as_bool() {
        Ok(())
    } else {
        Err(Error::MemoryError(unsafe { GetLastError() }))
    }
}

/// Reads data from an area of memory in a specified process.
///
/// # Errors
/// If the function fails, `Error::MemoryError` is returned.
pub fn read_process_memory(
    process_handle: Handle,
    base_address: LPCVOID,
    buffer: LPVOID,
    size: size_t,
    number_of_bytes_written: *mut size_t,
) -> Result<(), Error> {
    let res = unsafe { ReadProcessMemory(process_handle, base_address, buffer, size, number_of_bytes_written) };
    if res.as_bool() {
        Ok(())
    } else {
        Err(Error::MemoryError(unsafe { GetLastError() }))
    }
}

/// Retrieves the address of an exported function or variable from the specified
/// dynamic-link library (DLL).
///
/// # Errors
/// If the function fails, `Error::ProcessAddress` is returned.
pub fn get_proc_address(hmodule: HINSTANCE, lpprocname: &str) -> Result<usize, Error> {
    let function_address =
        unsafe { GetProcAddress(hmodule, lpprocname) }.ok_or_else(|| Error::ProcessAddress(unsafe { GetLastError() }))?;

    Ok(function_address as usize)
}

/// Reserves, commits, or changes the state of a region of memory within the
/// virtual address space of a specified process. The function initializes the
/// memory it allocates to zero.
///
/// If you provide none to `address` we will default it to a null pointer.
///
/// # Errors
/// If the function fails, `Error::MemoryError` is returned.
pub fn virtual_alloc_ex(
    handle: Handle,
    address: Option<*mut c_void>,
    size: usize,
    allocation_type: VirtualAllocationType,
    protection_flags: PageProtectionFlags,
) -> Result<*mut c_void, Error> {
    let res = unsafe { VirtualAllocEx(handle, address.unwrap_or(null_mut()), size, allocation_type, protection_flags) };

    if res.is_null() {
        Err(Error::MemoryError(unsafe { GetLastError() }))
    } else {
        Ok(res)
    }
}

/// Releases, decommits, or releases and decommits a region of memory within the
/// virtual address space of a specified process.
///
/// # Errors
/// If the function fails, `Error::MemoryError` is returned.
pub fn virtual_free_ex(
    process_handle: Handle,
    address: *mut c_void,
    size: usize,
    virtual_free_type: VirtualFreeType,
) -> Result<(), Error> {
    let result = unsafe { VirtualFreeEx(process_handle, address, size, virtual_free_type) };
    if result.as_bool() {
        Ok(())
    } else {
        Err(Error::MemoryError(unsafe { GetLastError() }))
    }
}

/// Disables the `DLL_THREAD_ATTACH` and `DLL_THREAD_DETACH` notifications for
/// the specified dynamic-link library (DLL). This can reduce the size of the
/// working set for some applications.
///
/// # Errors
/// If the function fails, `Error::Handle` is returned.
pub fn disable_thread_library_calls(module_handle: HandleInstance) -> Result<(), Error> {
    let result = unsafe { DisableThreadLibraryCalls(module_handle) };
    if result.as_bool() {
        Ok(())
    } else {
        Err(Error::Handle(unsafe { GetLastError() }))
    }
}

/// Will return a PID for the Handle it is given.
///
/// # Errors
/// On a PID of zero will return an error.
pub fn get_process_id(handle: Handle) -> Result<u32, Error> {
    let result = unsafe { GetProcessId(handle) };

    if result == 0 {
        Err(Error::Handle(unsafe { GetLastError() }))
    } else {
        Ok(result)
    }
}
