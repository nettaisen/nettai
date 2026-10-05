--!strict
-- LifeSync (EXE5 chip 0x01c): a cut-in chip that, outside link and boss-ranked battles, sets the HP of side 1's
-- combatants to that of the enemy two panels ahead, at most their own max HP: EXE5's controller (controller.luau, the
-- dimming handler's subtype 39, EXE6's reworked) with the shared marker (@exelib/lifesync/marker) in EXE5's hit sparks.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local lifesync = require("@self/controller")

return define.chip {
    id = "lifesync",
    codes = { "B", "R", "S", "*" },
    element = "null",
    family = "cursor",
    class = "standard",
    rarity = 1,
    mb = 12,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    dimming = lifesync.hook,
}
