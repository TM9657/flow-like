/// Defines permissions for Personal Access Tokens (PATs). Currently not implemented. TODO: Implement PAT permissions.
use bitflags::bitflags;

bitflags! {
    /// Represents a set of flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct PatPermission: i64 {
        const All             =   0b00000000_00000000_00000001;
        const Projects        =   0b00000000_00000000_00000010;
        const Teams           =   0b00000000_00000000_00000100;
        const Notifications   =   0b00000000_00000000_00001000;
        const Store           =   0b00000000_00000000_00010000;
        const Billing         =   0b00000000_00000000_00100000;
    }

}

pub fn has_pat_permission(permissions: &PatPermission, permission: PatPermission) -> bool {
    permissions.contains(permission) || permissions.contains(PatPermission::All)
}
