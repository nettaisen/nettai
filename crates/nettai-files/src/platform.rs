//! Each system's own, by concern: its folders ([`folders`]), its stowed files ([`stowed`]), and the browser's files
//! ([`tree`]: elsewhere they are `std::fs`'s). Each concern chooses its system in its module.

pub(crate) mod folders;
pub(crate) mod stowed;
// (In tests anywhere too: the tree's tests run on the desktop.)
#[cfg(any(web, test))]
#[cfg_attr(not(web), allow(dead_code))]
pub(crate) mod tree;
