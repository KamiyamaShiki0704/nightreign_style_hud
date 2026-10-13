//! Process-resident ownership for five actual ilhook redirects. Installation/ABI remain experimental.
use ilhook::x64::{
    CallbackOption, ClosureHookPoint, HookFlags, Registers, hook_closure_jmp_back,
    hook_closure_jmp_to_ret,
};
use shikifsrs_elden_ring::{Ww171File, hud_hook_sites::HudInlineHookSite};
use shikifsrs_runtime::{
    Error, Result,
    code_patches::{CodePatchSite, OwnedCodePatches, PreparedCodeSites},
    memory::{AddressSpace, CurrentProcess},
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
static READY: AtomicBool = AtomicBool::new(false);
struct HookOwner {
    _ticket: ClosureHookPoint<'static>,
}
// SAFETY: The only constructors below capture a Copy function pointer, using thread-safe static
// state. The third-party erased closure omits Send in its trait, but these actual captures are Send.
// Tickets never leave this process-resident journal, and are never dropped/uninstalled on a thread.
unsafe impl Send for HookOwner {}
struct Bootstrap {
    base: u64,
    sites: [HudInlineHookSite; 5],
    prepared: Option<PreparedCodeSites>,
    owners: Vec<(usize, HookOwner)>,
    attempted: [bool; 5],
    failed: bool,
    manifest: Option<Arc<OwnedCodePatches>>,
}
static BOOTSTRAP: Mutex<Option<Bootstrap>> = Mutex::new(None);
fn rejected(reason: &'static str) -> Error {
    Error::NativeOperationRejected(reason)
}
/// # Safety
/// Caller proves serialized exclusive installation outside loader lock; the host DLL stays pinned.
pub unsafe fn prepare() -> Result<()> {
    let (bytes, base) = crate::sdk_host::binding()?;
    let file = Ww171File::verify(&bytes)?;
    let sites = file.hud_inline_hook_sites();
    let plans = sites.map(|s| CodePatchSite {
        evidence: s.evidence,
        rva: s.rva,
        length: 14,
    });
    let prepared = PreparedCodeSites::prepare(file.file(), &CurrentProcess, base, &plans)?;
    let mut slot = BOOTSTRAP
        .lock()
        .map_err(|_| rejected("hook bootstrap poisoned"))?;
    if slot.is_some() {
        return Err(rejected("hook preparation already claimed"));
    }
    *slot = Some(Bootstrap {
        base,
        sites,
        prepared: Some(prepared),
        owners: Vec::with_capacity(5),
        attempted: [false; 5],
        failed: false,
        manifest: None,
    });
    Ok(())
}
pub fn address(index: usize) -> Result<usize> {
    let guard = BOOTSTRAP
        .lock()
        .map_err(|_| rejected("hook bootstrap poisoned"))?;
    let slot = guard.as_ref().ok_or(rejected("hooks not prepared"))?;
    let site = slot
        .sites
        .get(index)
        .ok_or(rejected("invalid hook index"))?;
    usize::try_from(slot.base + u64::from(site.rva)).map_err(|_| Error::AddressOverflow)
}
fn site(slot: &mut Bootstrap, index: usize) -> Result<usize> {
    if slot.failed || index >= 5 || slot.attempted[index] {
        return Err(rejected("hook attempt rejected"));
    }
    slot.attempted[index] = true;
    usize::try_from(slot.base + u64::from(slot.sites[index].rva))
        .map_err(|_| Error::AddressOverflow)
}
/// # Safety
/// Callback retains the actual hook register ABI and argument graph through each call.
pub unsafe fn install_back(index: usize, callback: fn(*mut Registers)) -> Result<()> {
    let mut guard = BOOTSTRAP
        .lock()
        .map_err(|_| rejected("hook bootstrap poisoned"))?;
    let slot = guard.as_mut().ok_or(rejected("hooks not prepared"))?;
    let address = site(slot, index)?;
    // SAFETY: Exact fixed site has been validated; caller supplies native quiescence and callback ABI.
    let result = unsafe {
        hook_closure_jmp_back(
            address,
            move |regs| {
                if READY.load(Ordering::Acquire) && !regs.is_null() {
                    callback(regs);
                }
            },
            CallbackOption::None,
            HookFlags::empty(),
        )
    };
    match result {
        Ok(ticket) => {
            slot.owners.push((index, HookOwner { _ticket: ticket }));
            Ok(())
        }
        Err(_) => {
            slot.failed = true;
            Err(rejected(
                "inline hook installation failed; previous owners retained",
            ))
        }
    }
}
/// # Safety
/// Callback's returned destination and original trampoline must satisfy the actual native ABI.
pub unsafe fn install_ret(
    index: usize,
    callback: fn(*mut Registers, usize) -> usize,
) -> Result<()> {
    let mut guard = BOOTSTRAP
        .lock()
        .map_err(|_| rejected("hook bootstrap poisoned"))?;
    let slot = guard.as_mut().ok_or(rejected("hooks not prepared"))?;
    let address = site(slot, index)?;
    // SAFETY: Same fixed-site/install contract; forwarding preserves the original until publication.
    let result = unsafe {
        hook_closure_jmp_to_ret(
            address,
            move |regs, original| {
                if READY.load(Ordering::Acquire) && !regs.is_null() {
                    callback(regs, original)
                } else {
                    original
                }
            },
            CallbackOption::None,
            HookFlags::empty(),
        )
    };
    match result {
        Ok(ticket) => {
            slot.owners.push((index, HookOwner { _ticket: ticket }));
            Ok(())
        }
        Err(_) => {
            slot.failed = true;
            Err(rejected(
                "inline hook installation failed; previous owners retained",
            ))
        }
    }
}
fn owned_redirect(original: &[u8], actual: &[u8], address: u64) -> Result<u64> {
    if original.len() != 14 || actual.len() != 14 {
        return Err(rejected("owned redirect length changed"));
    }
    if actual[0] == 0xe9 && actual[5..] == original[5..] {
        let delta = i32::from_le_bytes(actual[1..5].try_into().unwrap()) as i64;
        return address
            .checked_add(5)
            .and_then(|v| v.checked_add_signed(delta))
            .ok_or(Error::AddressOverflow);
    }
    if actual[..6] == [0xff, 0x25, 0, 0, 0, 0] {
        return Ok(u64::from_le_bytes(actual[6..14].try_into().unwrap()));
    }
    Err(rejected(
        "just-installed owned ilhook redirect encoding differs",
    ))
}
/// # Safety
/// Caller retains all installed redirects exclusively while declaring these exact owner-produced
/// bytes. No arbitrary pre-existing replacement is accepted: every site was original before this
/// batch and has a successful retained installer ticket. All future closure/graph contracts remain.
pub unsafe fn publish() -> Result<()> {
    let mut guard = BOOTSTRAP
        .lock()
        .map_err(|_| rejected("hook bootstrap poisoned"))?;
    let slot = guard.as_mut().ok_or(rejected("hooks not prepared"))?;
    if slot.failed || slot.owners.len() != 5 || !slot.attempted.iter().all(|v| *v) {
        return Err(rejected("incomplete hook group; partial owners retained"));
    }
    let (bytes, _) = crate::sdk_host::binding()?;
    let file = Ww171File::verify(&bytes)?;
    let mut declarations = Vec::with_capacity(5);
    for site in slot.sites {
        let mut actual = vec![0; 14];
        let address = slot.base + u64::from(site.rva);
        CurrentProcess.read_exact(address, &mut actual)?;
        let original = file.file().image().read_rva(site.rva, 14)?;
        let destination = owned_redirect(original, &actual, address)?;
        if destination == 0 || destination == address {
            return Err(rejected("invalid owned trampoline destination"));
        }
        declarations.push((site.rva, actual));
    }
    let prepared = slot
        .prepared
        .take()
        .ok_or(rejected("hook manifest already attempted"))?
        .declare_replacements(&declarations)?;
    // SAFETY: Every replacement belongs to this retained, pinned, just-installed hook batch.
    let manifest = Arc::new(unsafe { prepared.into_owned() });
    slot.manifest = Some(manifest.clone());
    crate::sdk_host::publish_patches(manifest)?;
    READY.store(true, Ordering::Release);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owned_redirect_keeps_near_tail_and_refuses_foreign_or_overflowing_encoding() {
        let original = [0xcc; 14];
        let mut near = original;
        near[0] = 0xe9;
        near[1..5].copy_from_slice(&(-12i32).to_le_bytes());
        assert_eq!(owned_redirect(&original, &near, 100).unwrap(), 93);
        near[13] = 0;
        assert!(owned_redirect(&original, &near, 100).is_err());
        let mut far = [0; 14];
        far[..6].copy_from_slice(&[0xff, 0x25, 0, 0, 0, 0]);
        far[6..].copy_from_slice(&12345u64.to_le_bytes());
        assert_eq!(owned_redirect(&original, &far, 100).unwrap(), 12345);
        assert!(owned_redirect(&original, &original, 100).is_err());
        assert!(owned_redirect(&original, &far[..13], 100).is_err());
        near = original;
        near[0] = 0xe9;
        near[1..5].copy_from_slice(&i32::MIN.to_le_bytes());
        assert!(owned_redirect(&original, &near, 100).is_err());
    }
}
