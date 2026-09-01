//! Pure config-editing operations the GUI drives (profile CRUD, binding
//! upsert/remove). Kept here — not in the Tauri bridge — so the correctness of
//! these mutations is unit-tested and the commands stay dumb.

use crate::config::{Config, PadBinding, Profile};
use crate::midi::DEFAULT_BASE_NOTE;

/// A reason an edit could not be applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    ProfileNotFound(String),
    ProfileExists(String),
}

/// Add a new empty profile. Errors if the id already exists. If no profile was
/// active, the new one becomes active.
pub fn add_profile(cfg: &mut Config, id: &str, name: &str) -> Result<(), EditError> {
    if cfg.profiles.iter().any(|p| p.id == id) {
        return Err(EditError::ProfileExists(id.to_string()));
    }
    cfg.profiles.push(Profile {
        id: id.to_string(),
        name: name.to_string(),
        base_note: DEFAULT_BASE_NOTE,
        bindings: Vec::new(),
    });
    if cfg.active.is_none() {
        cfg.active = Some(id.to_string());
    }
    Ok(())
}

/// Delete a profile by id. If it was active, active moves to the first remaining
/// profile (or `None`). Errors if the id is not found.
pub fn delete_profile(cfg: &mut Config, id: &str) -> Result<(), EditError> {
    let before = cfg.profiles.len();
    cfg.profiles.retain(|p| p.id != id);
    if cfg.profiles.len() == before {
        return Err(EditError::ProfileNotFound(id.to_string()));
    }
    if cfg.active.as_deref() == Some(id) {
        cfg.active = cfg.profiles.first().map(|p| p.id.clone());
    }
    Ok(())
}

/// Rename a profile. Errors if the id is not found.
pub fn rename_profile(cfg: &mut Config, id: &str, new_name: &str) -> Result<(), EditError> {
    profile_mut(cfg, id)?.name = new_name.to_string();
    Ok(())
}

/// Set the active profile. Errors if the id is not found.
pub fn set_active(cfg: &mut Config, id: &str) -> Result<(), EditError> {
    if !cfg.profiles.iter().any(|p| p.id == id) {
        return Err(EditError::ProfileNotFound(id.to_string()));
    }
    cfg.active = Some(id.to_string());
    Ok(())
}

/// Insert or replace the binding at `(bank, cell)` in a profile.
pub fn upsert_binding(cfg: &mut Config, profile_id: &str, pad: PadBinding) -> Result<(), EditError> {
    let profile = profile_mut(cfg, profile_id)?;
    if let Some(existing) = profile
        .bindings
        .iter_mut()
        .find(|b| b.bank == pad.bank && b.cell == pad.cell)
    {
        *existing = pad;
    } else {
        profile.bindings.push(pad);
    }
    Ok(())
}

/// Remove the binding at `(bank, cell)` in a profile (no-op if none). Errors if
/// the profile is not found.
pub fn remove_binding(
    cfg: &mut Config,
    profile_id: &str,
    bank: u8,
    cell: u8,
) -> Result<(), EditError> {
    let profile = profile_mut(cfg, profile_id)?;
    profile.bindings.retain(|b| !(b.bank == bank && b.cell == cell));
    Ok(())
}

fn profile_mut<'a>(cfg: &'a mut Config, id: &str) -> Result<&'a mut Profile, EditError> {
    cfg.profiles
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or_else(|| EditError::ProfileNotFound(id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Binding, MacroStep, TriggerMode};
    use crate::midi::Color;

    fn chord(keys: &[&str]) -> Binding {
        Binding {
            trigger: TriggerMode::Tap,
            steps: vec![MacroStep::Chord {
                keys: keys.iter().map(|s| s.to_string()).collect(),
            }],
            color: Color(7),
        }
    }

    #[test]
    fn add_profile_appends_and_activates_first() {
        let mut cfg = Config::default();
        add_profile(&mut cfg, "p1", "One").unwrap();
        assert_eq!(cfg.profiles.len(), 1);
        assert_eq!(cfg.profiles[0].id, "p1");
        assert_eq!(cfg.profiles[0].name, "One");
        assert_eq!(cfg.profiles[0].base_note, DEFAULT_BASE_NOTE);
        assert_eq!(cfg.active.as_deref(), Some("p1")); // first becomes active
        // A second add does not steal active.
        add_profile(&mut cfg, "p2", "Two").unwrap();
        assert_eq!(cfg.active.as_deref(), Some("p1"));
    }

    #[test]
    fn add_profile_rejects_duplicate_id() {
        let mut cfg = Config::default();
        add_profile(&mut cfg, "p1", "One").unwrap();
        assert_eq!(
            add_profile(&mut cfg, "p1", "Dup"),
            Err(EditError::ProfileExists("p1".into()))
        );
        assert_eq!(cfg.profiles.len(), 1);
    }

    #[test]
    fn delete_profile_reassigns_active() {
        let mut cfg = Config::default();
        add_profile(&mut cfg, "p1", "One").unwrap();
        add_profile(&mut cfg, "p2", "Two").unwrap();
        set_active(&mut cfg, "p1").unwrap();
        delete_profile(&mut cfg, "p1").unwrap();
        assert_eq!(cfg.profiles.len(), 1);
        assert_eq!(cfg.active.as_deref(), Some("p2")); // moved to remaining
    }

    #[test]
    fn delete_last_profile_clears_active() {
        let mut cfg = Config::default();
        add_profile(&mut cfg, "p1", "One").unwrap();
        delete_profile(&mut cfg, "p1").unwrap();
        assert!(cfg.profiles.is_empty());
        assert_eq!(cfg.active, None);
    }

    #[test]
    fn delete_missing_profile_errors() {
        let mut cfg = Config::default();
        assert_eq!(
            delete_profile(&mut cfg, "nope"),
            Err(EditError::ProfileNotFound("nope".into()))
        );
    }

    #[test]
    fn rename_and_set_active_report_missing() {
        let mut cfg = Config::default();
        add_profile(&mut cfg, "p1", "One").unwrap();
        rename_profile(&mut cfg, "p1", "Renamed").unwrap();
        assert_eq!(cfg.profiles[0].name, "Renamed");
        assert_eq!(
            rename_profile(&mut cfg, "x", "y"),
            Err(EditError::ProfileNotFound("x".into()))
        );
        assert_eq!(
            set_active(&mut cfg, "x"),
            Err(EditError::ProfileNotFound("x".into()))
        );
    }

    #[test]
    fn upsert_binding_inserts_then_replaces() {
        let mut cfg = Config::default();
        add_profile(&mut cfg, "p1", "One").unwrap();
        upsert_binding(
            &mut cfg,
            "p1",
            PadBinding { bank: 0, cell: 0, binding: chord(&["a"]) },
        )
        .unwrap();
        assert_eq!(cfg.profiles[0].bindings.len(), 1);
        // Replacing the same (bank,cell) updates in place, not appends.
        upsert_binding(
            &mut cfg,
            "p1",
            PadBinding { bank: 0, cell: 0, binding: chord(&["b"]) },
        )
        .unwrap();
        assert_eq!(cfg.profiles[0].bindings.len(), 1);
        assert_eq!(cfg.profiles[0].bindings[0].binding, chord(&["b"]));
    }

    #[test]
    fn upsert_binding_missing_profile_errors() {
        let mut cfg = Config::default();
        assert_eq!(
            upsert_binding(
                &mut cfg,
                "nope",
                PadBinding { bank: 0, cell: 0, binding: chord(&["a"]) }
            ),
            Err(EditError::ProfileNotFound("nope".into()))
        );
    }

    #[test]
    fn remove_binding_deletes_only_that_cell() {
        let mut cfg = Config::default();
        add_profile(&mut cfg, "p1", "One").unwrap();
        upsert_binding(&mut cfg, "p1", PadBinding { bank: 0, cell: 0, binding: chord(&["a"]) }).unwrap();
        upsert_binding(&mut cfg, "p1", PadBinding { bank: 0, cell: 1, binding: chord(&["b"]) }).unwrap();
        remove_binding(&mut cfg, "p1", 0, 0).unwrap();
        assert_eq!(cfg.profiles[0].bindings.len(), 1);
        assert_eq!(cfg.profiles[0].bindings[0].cell, 1);
    }
}
