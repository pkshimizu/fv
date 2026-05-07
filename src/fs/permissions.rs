use std::fs::Permissions;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Debug)]
pub struct VPermissions {
    permissions: Permissions,
}

#[cfg(unix)]
const RWX_BITS: [(u32, char); 9] = [
    (0o400, 'r'),
    (0o200, 'w'),
    (0o100, 'x'),
    (0o040, 'r'),
    (0o020, 'w'),
    (0o010, 'x'),
    (0o004, 'r'),
    (0o002, 'w'),
    (0o001, 'x'),
];

impl VPermissions {
    pub fn new(permissions: Permissions) -> VPermissions {
        Self { permissions }
    }

    #[cfg(unix)]
    pub fn to_rwx_string(&self) -> String {
        let mode = self.permissions.mode();
        RWX_BITS
            .iter()
            .map(|&(mask, ch)| if mode & mask != 0 { ch } else { '-' })
            .collect()
    }

    #[cfg(not(unix))]
    pub fn to_rwx_string(&self) -> String {
        if self.permissions.readonly() {
            "readonly".to_string()
        } else {
            "read-write".to_string()
        }
    }
}
