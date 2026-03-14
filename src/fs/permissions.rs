use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;

#[derive(Debug)]
pub struct VPermissions {
    pub permissions: Permissions,
}

impl VPermissions {
    pub fn new(permissions: Permissions) -> VPermissions {
        Self { permissions }
    }

    #[allow(dead_code)]
    pub fn owner_read(&self) -> bool {
        self.permissions.mode() & 0o400 != 0
    }

    #[allow(dead_code)]
    pub fn owner_write(&self) -> bool {
        self.permissions.mode() & 0o200 != 0
    }

    #[allow(dead_code)]
    pub fn owner_exec(&self) -> bool {
        self.permissions.mode() & 0o100 != 0
    }

    #[allow(dead_code)]
    pub fn group_read(&self) -> bool {
        self.permissions.mode() & 0o040 != 0
    }

    #[allow(dead_code)]
    pub fn group_write(&self) -> bool {
        self.permissions.mode() & 0o020 != 0
    }

    #[allow(dead_code)]
    pub fn group_exec(&self) -> bool {
        self.permissions.mode() & 0o010 != 0
    }

    #[allow(dead_code)]
    pub fn other_read(&self) -> bool {
        self.permissions.mode() & 0o004 != 0
    }

    #[allow(dead_code)]
    pub fn other_write(&self) -> bool {
        self.permissions.mode() & 0o002 != 0
    }

    #[allow(dead_code)]
    pub fn other_exec(&self) -> bool {
        self.permissions.mode() & 0o001 != 0
    }

    pub fn to_rwx_string(&self) -> String {
        let mode = self.permissions.mode();
        format!(
            "{}{}{}{}{}{}{}{}{}",
            if mode & 0o400 != 0 { "r" } else { "-" },
            if mode & 0o200 != 0 { "w" } else { "-" },
            if mode & 0o100 != 0 { "x" } else { "-" },
            if mode & 0o040 != 0 { "r" } else { "-" },
            if mode & 0o020 != 0 { "w" } else { "-" },
            if mode & 0o010 != 0 { "x" } else { "-" },
            if mode & 0o004 != 0 { "r" } else { "-" },
            if mode & 0o002 != 0 { "w" } else { "-" },
            if mode & 0o001 != 0 { "x" } else { "-" },
        )
    }
}
