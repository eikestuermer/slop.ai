//! Project-level access control list.
//!
//! ACLs are themselves Automerge documents stored alongside the timeline
//! document. They list public keys (hex) and roles. The sync server
//! validates every incoming sync message against the ACL.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// Per-project role.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Read-only.
    Viewer,
    /// Read + comment.
    Reviewer,
    /// Read + write.
    Editor,
    /// Read + write + manage ACL.
    Owner,
}

impl Role {
    /// Can this role mutate the timeline?
    pub fn can_edit(&self) -> bool {
        matches!(self, Role::Editor | Role::Owner)
    }
    /// Can this role add comments?
    pub fn can_comment(&self) -> bool {
        !matches!(self, Role::Viewer)
    }
    /// Can this role grant/revoke other roles?
    pub fn can_manage(&self) -> bool {
        matches!(self, Role::Owner)
    }
}

/// In-memory ACL.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Acl {
    /// pubkey hex -> role.
    pub roles: BTreeMap<String, Role>,
}

/// Errors.
#[derive(Debug, Error)]
pub enum AclError {
    /// Caller doesn't have permission for the requested action.
    #[error("permission denied for {pubkey} (role {role:?}): {action}")]
    Denied {
        /// Actor pubkey hex.
        pubkey: String,
        /// Their current role.
        role: Role,
        /// What they tried to do.
        action: String,
    },
    /// Unknown principal.
    #[error("pubkey {0} is not in the ACL")]
    Unknown(String),
}

impl Acl {
    /// Look up a role.
    pub fn role_of(&self, pubkey_hex: &str) -> Option<Role> {
        self.roles.get(pubkey_hex).copied()
    }

    /// Authorize an edit operation. Returns Ok if the actor has Editor or Owner.
    pub fn authorize_edit(&self, pubkey_hex: &str) -> Result<(), AclError> {
        let role = self
            .role_of(pubkey_hex)
            .ok_or_else(|| AclError::Unknown(pubkey_hex.to_string()))?;
        if !role.can_edit() {
            return Err(AclError::Denied {
                pubkey: pubkey_hex.to_string(),
                role,
                action: "edit".into(),
            });
        }
        Ok(())
    }

    /// Authorize a manage (ACL change) operation.
    pub fn authorize_manage(&self, pubkey_hex: &str) -> Result<(), AclError> {
        let role = self
            .role_of(pubkey_hex)
            .ok_or_else(|| AclError::Unknown(pubkey_hex.to_string()))?;
        if !role.can_manage() {
            return Err(AclError::Denied {
                pubkey: pubkey_hex.to_string(),
                role,
                action: "manage".into(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_helpers_match_expected_capabilities() {
        assert!(Role::Owner.can_edit());
        assert!(Role::Owner.can_manage());
        assert!(Role::Editor.can_edit());
        assert!(!Role::Editor.can_manage());
        assert!(Role::Reviewer.can_comment());
        assert!(!Role::Reviewer.can_edit());
        assert!(!Role::Viewer.can_comment());
    }

    #[test]
    fn authorize_denies_viewer_edit() {
        let mut acl = Acl::default();
        acl.roles.insert("aa".into(), Role::Viewer);
        assert!(matches!(
            acl.authorize_edit("aa"),
            Err(AclError::Denied { .. })
        ));
    }

    #[test]
    fn authorize_unknown_returns_unknown() {
        let acl = Acl::default();
        assert!(matches!(
            acl.authorize_edit("zz"),
            Err(AclError::Unknown(_))
        ));
    }

    #[test]
    fn owner_can_edit_and_manage() {
        let mut acl = Acl::default();
        acl.roles.insert("owner".into(), Role::Owner);
        assert!(acl.authorize_edit("owner").is_ok());
        assert!(acl.authorize_manage("owner").is_ok());
    }

    #[test]
    fn editor_can_edit_but_not_manage() {
        let mut acl = Acl::default();
        acl.roles.insert("ed".into(), Role::Editor);
        assert!(acl.authorize_edit("ed").is_ok());
        assert!(matches!(
            acl.authorize_manage("ed"),
            Err(AclError::Denied { .. })
        ));
    }

    #[test]
    fn reviewer_can_comment_but_not_edit() {
        let mut acl = Acl::default();
        acl.roles.insert("r".into(), Role::Reviewer);
        assert!(Role::Reviewer.can_comment());
        assert!(matches!(
            acl.authorize_edit("r"),
            Err(AclError::Denied { .. })
        ));
    }
}
