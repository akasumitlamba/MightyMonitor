//! Owned virtual-output lifecycle for a future native driver adapter.
//! No driver is invoked by this module or selected by the application yet.
use std::io;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
}
impl Mode {
    pub fn validate(self) -> io::Result<()> {
        if !(640..=8192).contains(&self.width)
            || !(480..=8192).contains(&self.height)
            || self.width % 2 != 0
            || self.height % 2 != 0
            || !(1..=120).contains(&self.refresh_hz)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unsupported virtual display mode",
            ));
        }
        Ok(())
    }
}
/// The OS identity returned by IddCxMonitorArrival, not an enumeration index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputId {
    pub adapter_luid: i64,
    pub target_id: u32,
}
/// An opaque driver-owned reference; it must be scoped to the creating connection.
#[derive(Debug, PartialEq, Eq)]
pub struct CreatedOutput {
    pub lease_id: u64,
    pub output: OutputId,
}
/// Implementors must atomically allocate one owned monitor, never replace the
/// global monitor list. Failed create must roll itself back; remove is idempotent
/// and scoped to this connection's lease. Connection loss must reap its monitors.
/// All native operations must have deadlines; Drop is not a crash-cleanup guarantee.
pub trait Backend {
    fn create(&mut self, mode: Mode) -> io::Result<CreatedOutput>;
    fn validate(&mut self, created: &CreatedOutput, mode: Mode) -> io::Result<()>;
    fn remove(&mut self, created: &CreatedOutput) -> io::Result<()>;
}
/// Keeps the exact owned output alive until capture and its input are dropped.
/// The caller must finish capture/input before stop or dropping this owner.
pub struct OwnedOutput<B: Backend> {
    backend: B,
    created: Option<CreatedOutput>,
    mode: Mode,
}
impl<B: Backend> OwnedOutput<B> {
    pub fn create(mut backend: B, mode: Mode) -> io::Result<Self> {
        mode.validate()?;
        let created = backend.create(mode)?;
        let mut owned = Self {
            backend,
            created: Some(created),
            mode,
        };
        // Arm cleanup before validation: rollback happens even if identity,
        // topology or mode verification fails (or the adapter unwinds).
        owned.validate()?;
        Ok(owned)
    }
    pub fn identity(&self) -> io::Result<OutputId> {
        self.created
            .as_ref()
            .map(|created| created.output)
            .ok_or_else(|| io::Error::other("Virtual display has stopped"))
    }
    pub fn validate(&mut self) -> io::Result<()> {
        let created = self
            .created
            .as_ref()
            .ok_or_else(|| io::Error::other("Virtual display has stopped"))?;
        self.backend.validate(created, self.mode)
    }
    /// Retains ownership on failure so the caller can retry cleanup. A driver
    /// adapter must also reap the lease when its connection is finally closed.
    pub fn stop(&mut self) -> io::Result<()> {
        if let Some(created) = &self.created {
            self.backend.remove(created)?;
            self.created = None;
        }
        Ok(())
    }
}
impl<B: Backend> Drop for OwnedOutput<B> {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(target_os = "windows")]
impl OwnedOutput<crate::DriverBackend> {
    /// Returns only the exact validated screen created by this owner's driver.
    pub fn display(&mut self) -> io::Result<crate::Display> {
        self.validate()?;
        self.backend.display()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    #[derive(Default)]
    struct State {
        next: u64,
        live: Vec<u64>,
        removed: Vec<u64>,
        invalid: bool,
        fail_remove: bool,
    }
    struct Fake(Arc<Mutex<State>>);
    impl Backend for Fake {
        fn create(&mut self, _: Mode) -> io::Result<CreatedOutput> {
            let mut state = self.0.lock().unwrap();
            state.next += 1;
            let id = state.next;
            state.live.push(id);
            Ok(CreatedOutput {
                lease_id: id,
                output: OutputId {
                    adapter_luid: 11,
                    target_id: id as u32,
                },
            })
        }
        fn validate(&mut self, created: &CreatedOutput, _: Mode) -> io::Result<()> {
            let state = self.0.lock().unwrap();
            if state.invalid || !state.live.contains(&created.lease_id) {
                Err(io::Error::other("identity changed"))
            } else {
                Ok(())
            }
        }
        fn remove(&mut self, created: &CreatedOutput) -> io::Result<()> {
            let mut state = self.0.lock().unwrap();
            if state.fail_remove {
                state.fail_remove = false;
                return Err(io::Error::other("temporary cleanup failure"));
            }
            state.live.retain(|&id| id != created.lease_id);
            state.removed.push(created.lease_id);
            Ok(())
        }
    }
    fn mode() -> Mode {
        Mode {
            width: 1920,
            height: 1080,
            refresh_hz: 60,
        }
    }
    #[test]
    fn independent_owners_preserve_other_outputs_and_cleanup_once() {
        let state = Arc::new(Mutex::new(State {
            next: 99,
            live: vec![42],
            ..Default::default()
        }));
        let mut first = OwnedOutput::create(Fake(state.clone()), mode()).unwrap();
        let second = OwnedOutput::create(Fake(state.clone()), mode()).unwrap();
        assert_ne!(first.identity().unwrap(), second.identity().unwrap());
        first.stop().unwrap();
        first.stop().unwrap();
        drop(first);
        assert_eq!(state.lock().unwrap().live, vec![42, 101]);
        drop(second);
        assert_eq!(state.lock().unwrap().live, vec![42]);
        assert_eq!(state.lock().unwrap().removed, vec![100, 101]);
    }
    #[test]
    fn validation_failure_rolls_back_only_the_new_owned_output() {
        let state = Arc::new(Mutex::new(State {
            live: vec![42],
            invalid: true,
            ..Default::default()
        }));
        assert!(OwnedOutput::create(Fake(state.clone()), mode()).is_err());
        assert_eq!(state.lock().unwrap().live, vec![42]);
        assert_eq!(state.lock().unwrap().removed, vec![1]);
    }
    #[test]
    fn cleanup_failure_retains_lease_for_retry_and_changed_identity_is_rejected() {
        let state = Arc::new(Mutex::new(State::default()));
        let mut owned = OwnedOutput::create(Fake(state.clone()), mode()).unwrap();
        state.lock().unwrap().invalid = true;
        assert!(owned.validate().is_err());
        state.lock().unwrap().fail_remove = true;
        assert!(owned.stop().is_err());
        assert!(owned.identity().is_ok());
        owned.stop().unwrap();
        assert!(owned.identity().is_err());
        assert!(owned.validate().is_err());
        drop(owned);
        assert_eq!(state.lock().unwrap().removed, vec![1]);
    }
    #[test]
    fn invalid_modes_never_create_a_monitor() {
        let state = Arc::new(Mutex::new(State::default()));
        for bad in [
            Mode { width: 0, ..mode() },
            Mode {
                height: 1081,
                ..mode()
            },
            Mode {
                refresh_hz: 121,
                ..mode()
            },
        ] {
            assert!(OwnedOutput::create(Fake(state.clone()), bad).is_err());
        }
        assert_eq!(state.lock().unwrap().next, 0);
    }
}
