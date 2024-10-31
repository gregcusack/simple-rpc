use {
    agave_geyser_plugin_interface::geyser_plugin_interface::FfiContactInfo,
    log::*,
    memmap2::MmapOptions,
    std::{
        fs::OpenOptions,
        path::Path,
        sync::atomic::{AtomicU64, Ordering},
        mem::size_of,
        thread,
        time::Duration,
    },
    solana_program::pubkey::Pubkey,
};

pub const RUST_LOG_FILTER: &str = "info";
const SHM_PATH: &str = "/tmp/ffi_contact_info_shm";
const BUFFER_CAPACITY: usize = 10_000;
static ENTRY_SIZE: usize = size_of::<FfiContactInfo>();

fn main() {
    solana_logger::setup_with_default(RUST_LOG_FILTER); // Ensure logging is initialized

    // Open the shared memory file
    let shm_path = Path::new(SHM_PATH);
    let file = OpenOptions::new()
        .read(true)
        .open(&shm_path)
        .expect("Failed to open shared memory file");

    // Memory-map the file
    let mmap = unsafe {
        MmapOptions::new()
            .map(&file)
            .expect("Failed to map shared memory")
    };

    // Get pointers to head and tail
    let head_ptr = &mmap[..size_of::<u64>()] as *const [u8] as *const AtomicU64;
    let tail_ptr = &mmap[size_of::<u64>()..size_of::<u64>() * 2] as *const [u8] as *const AtomicU64;

    let head = unsafe { &*head_ptr };
    let tail = unsafe { &*tail_ptr };

    loop {
        // Read current head and tail
        let current_head = head.load(Ordering::SeqCst);
        let mut current_tail = tail.load(Ordering::SeqCst);

        // Calculate the number of new entries
        let entries_available = current_head - current_tail;

        if entries_available > 0 {
            // Read new entries
            for _ in 0..entries_available {
                let index = current_tail % BUFFER_CAPACITY as u64;
                let offset = size_of::<u64>() * 2 + index as usize * ENTRY_SIZE;

                // Read FfiContactInfo from shared memory
                let ptr = &mmap[offset..offset + ENTRY_SIZE] as *const [u8] as *const FfiContactInfo;
                let ffi_ci = unsafe { (*ptr).clone() };

                // Process or print the FfiContactInfo
                info!("new ffi_ci -> pk: {}, wc: {}, sv: {}", Pubkey::from(ffi_ci.pubkey), ffi_ci.wallclock, ffi_ci.shred_version);

                current_tail += 1;
            }

            // Update tail
            tail.store(current_tail, Ordering::SeqCst);
        }

        // Sleep for 5 seconds
        thread::sleep(Duration::from_secs(1));
    }
}
