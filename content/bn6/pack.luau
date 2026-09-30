--!strict
-- The bn6 content pack (docs/design/scripting.md). This spike carries one
-- vertical slice, GunDelSol: its action and the three object kinds it
-- spawns. Everything else is still the engine's built-in Rust.

return {
    objects = {
        require("./objects/attachment"),
        require("./objects/sun_beam"),
        require("./objects/hitbox"),
    },
    actions = {
        require("./chips/gun_del_sol"),
    },
}
