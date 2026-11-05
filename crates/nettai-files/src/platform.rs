//! Each system's own, by concern: its files ([`files`]), its folders ([`folders`]) and its stowed files
//! ([`stowed`]). Each concern chooses its system in its module.

pub(crate) mod files;
pub(crate) mod folders;
pub(crate) mod stowed;
