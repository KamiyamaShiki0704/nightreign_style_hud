//! Original current-version host shared by the four owned consumers.
//! Native phase/transitive graph lifetimes remain experimental host obligations.
use shikifsrs_elden_ring::{
    Ww171File,
    character_sets::CharacterIdentity,
    current_objects::CurrentObjects,
    objects::GameObjectCatalog,
    physics_queries::{CurrentLineQueries, LineQuery, NativeLineQueryCall},
};
use shikifsrs_runtime::{Error, Result, memory::CurrentProcess};
use std::{
    cell::RefCell,
    collections::VecDeque,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::HMODULE,
    System::LibraryLoader::{GetModuleHandleExW, GetModuleHandleW},
};
use windows::core::PCWSTR;
struct Host {
    bytes: Arc<[u8]>,
    base: u64,
}
static HOST: OnceLock<Host> = OnceLock::new();
static PATCHES: OnceLock<Arc<shikifsrs_runtime::code_patches::OwnedCodePatches>> = OnceLock::new();
static INITIALIZE: Mutex<()> = Mutex::new(());
static EPOCH: AtomicU64 = AtomicU64::new(1);
static OVERLAY: Mutex<Option<shikifsrs_overlay::OverlayTicket>> = Mutex::new(None);
#[derive(Default)]
struct Readers {
    current: Option<CurrentObjects>,
    patched: bool,
}
#[derive(Default)]
#[allow(dead_code)] // Query-bearing members compile this shared private host separately.
struct Queries {
    current: Option<QuerySession>,
    old: Vec<QuerySession>,
}
#[allow(dead_code)]
struct QuerySession {
    epoch: u64,
    patched: bool,
    access: CurrentLineQueries,
    receipts: VecDeque<NativeLineQueryCall>,
    failure: Option<NativeLineQueryCall>,
    pending: Option<(CharacterIdentity, LineQuery)>,
    blocked: bool,
}
thread_local! {
    static READERS:RefCell<Readers>=RefCell::new(Readers::default());
    static QUERIES:RefCell<Queries>=RefCell::new(Queries::default());
}
#[unsafe(no_mangle)]
pub extern "system" fn ShikiHudNotifyEpoch(value: u64) {
    if value != 0 {
        let _ = EPOCH.fetch_update(Ordering::AcqRel, Ordering::Acquire, |old| {
            (value > old).then_some(value)
        });
    }
}
pub fn epoch() -> u64 {
    EPOCH.load(Ordering::Acquire)
}
#[allow(dead_code)] // Shared by the task-bearing members, separately compiled DLLs.
pub fn binding() -> Result<(Arc<[u8]>, u64)> {
    let host = HOST
        .get()
        .ok_or(Error::NativeOperationRejected("SDK host not initialized"))?;
    Ok((host.bytes.clone(), host.base))
}
#[allow(dead_code)]
pub fn patches() -> Option<Arc<shikifsrs_runtime::code_patches::OwnedCodePatches>> {
    PATCHES.get().cloned()
}
#[allow(dead_code)]
pub fn publish_patches(
    patches: Arc<shikifsrs_runtime::code_patches::OwnedCodePatches>,
) -> Result<()> {
    let host = HOST
        .get()
        .ok_or(Error::NativeOperationRejected("SDK host not initialized"))?;
    Ww171File::verify(&host.bytes)?.bind_objects_with_patches(
        &CurrentProcess,
        host.base,
        Some(&patches),
    )?;
    PATCHES
        .set(patches)
        .map_err(|_| Error::NativeOperationRejected("HUD code manifest already published"))
}
pub fn initialize() -> std::result::Result<(), String> {
    let _guard = INITIALIZE.lock().map_err(|_| "SDK host lock poisoned")?;
    if HOST.get().is_some() {
        return Ok(());
    }
    let bytes: Arc<[u8]> = std::fs::read(std::env::current_exe().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?
        .into();
    let file = Ww171File::verify(&bytes).map_err(|e| e.to_string())?;
    let mut module = HMODULE::default();
    // SAFETY: PIN|FROM_ADDRESS accepts this resident function's address, no game call.
    unsafe {
        GetModuleHandleExW(
            5,
            PCWSTR(initialize as *const () as *const u16),
            &mut module,
        )
    }
    .map_err(|e| e.to_string())?;
    // SAFETY: Current host executable handle; no engine object lifetime is acquired.
    let base = unsafe { GetModuleHandleW(None) }
        .map_err(|e| e.to_string())?
        .0 as u64;
    file.bind_objects(&CurrentProcess, base)
        .map_err(|e| e.to_string())?;
    HOST.set(Host { bytes, base })
        .map_err(|_| "SDK host already initialized".to_string())
}
fn prepare() -> std::result::Result<(), String> {
    initialize()?;
    let start = Instant::now();
    loop {
        if with_objects(
            |objects, _| Ok(objects.window()?.is_some() && objects.cameras()?.is_some()),
        )
        .map_err(|e| e.to_string())?
        {
            return Ok(());
        }
        if start.elapsed() >= Duration::from_secs(30) {
            return Err("current ER renderer initialization timeout".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
pub fn start_overlay(
    factory: impl FnOnce() -> Box<dyn shikifsrs_overlay::hudhook::ImguiRenderLoop + Send + Sync>
    + Send
    + 'static,
) -> std::result::Result<(), String> {
    let mut retained = OVERLAY.lock().map_err(|_| "overlay lock poisoned")?;
    if retained.is_some() {
        return Err("overlay already attempted".into());
    }
    // SAFETY: Preparation pins this DLL before the one actual hook attempt outside loader lock.
    // Actual renderer/native graph and feature acceptance remain experimental obligations.
    let ticket =
        unsafe { shikifsrs_overlay::start_dx12(prepare, factory) }.map_err(|e| e.to_string())?;
    *retained = Some(ticket);
    Ok(())
}
pub fn with_objects<R>(
    read: impl FnOnce(&GameObjectCatalog<'_, '_, CurrentProcess>, u64) -> Result<R>,
) -> Result<R> {
    let host = HOST
        .get()
        .ok_or(Error::NativeOperationRejected("SDK host not initialized"))?;
    READERS.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| Error::NativeOperationRejected("recursive SDK observation"))?;
        let epoch = epoch();
        let patched = PATCHES.get().is_some();
        if slot.patched != patched || slot.current.as_ref().is_some_and(|v| v.epoch() != epoch) {
            slot.patched = patched;
            slot.current = None;
        }
        if slot.current.is_none() {
            // SAFETY: The actual callback retains the experimental exclusive readable graph;
            // this thread owns its reader. Epoch notification revokes previous observations.
            slot.current = Some(unsafe {
                CurrentObjects::bind(host.bytes.clone(), host.base, epoch, patches())
            }?);
        }
        slot.current.as_ref().unwrap().observe(read)
    })
}
#[allow(dead_code)]
pub fn cast_line(
    target: CharacterIdentity,
    filter: u32,
    origin: [f32; 3],
    delta: [f32; 3],
) -> Result<Option<[f32; 3]>> {
    let host = HOST
        .get()
        .ok_or(Error::NativeOperationRejected("SDK host not initialized"))?;
    QUERIES.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| Error::NativeOperationRejected("recursive line query"))?;
        let epoch = epoch();
        if target.epoch != epoch {
            return Err(Error::NativeOperationRejected("revoked ray exclusion"));
        }
        let patched = patches().is_some();
        if slot
            .current
            .as_ref()
            .is_some_and(|v| v.epoch != epoch || v.patched != patched)
        {
            if slot.old.len() == 64 {
                return Err(Error::NativeOperationRejected(
                    "query generation retention bound",
                ));
            }
            let old = slot.current.take().unwrap();
            slot.old.push(old);
        }
        if slot.current.is_none() {
            if slot.old.len() >= 64 {
                return Err(Error::NativeOperationRejected(
                    "query generation retention bound",
                ));
            }
            // SAFETY: Actual original render/callback host supplies the exclusive full query/
            // collision graph. Native execution remains experimental and is never used in tests.
            let access = unsafe {
                CurrentLineQueries::bind(host.bytes.clone(), host.base, epoch, patches())
            }?;
            slot.current = Some(QuerySession {
                epoch,
                patched,
                access,
                receipts: VecDeque::new(),
                failure: None,
                pending: None,
                blocked: false,
            });
        }
        let current = slot.current.as_mut().unwrap();
        if current.blocked || current.failure.is_some() {
            return Err(Error::NativeOperationRejected(
                "failed ray receipt retained; queries blocked",
            ));
        }
        let graph = current
            .access
            .observe()?
            .ok_or(Error::NativeOperationRejected("no query graph"))?;
        let excluded = current
            .access
            .observe_exclusion(target)?
            .ok_or(Error::NativeOperationRejected("no ray exclusion module"))?;
        let query = LineQuery::new(filter, origin, delta)?;
        // SAFETY: Exact thread-confined full query graph and exclusion were freshly copied;
        // host independently supplies native phase/heap/TLS/scene lifetime. Receipt is retained.
        current.pending = Some((target, query));
        current.blocked = true;
        let receipt = unsafe { current.access.cast_line(graph, query, Some(excluded)) }?;
        let result = receipt.hit_position();
        current.pending = None;
        if result.is_err() {
            current.failure = Some(receipt);
        } else {
            current.blocked = false;
            current.receipts.push_back(receipt);
            if current.receipts.len() > 64 {
                current.receipts.pop_front();
            }
        }
        result
    })
}
