--!strict
-- LifeSync: a cut-in chip that, outside link battles, sets the HP of the
-- other side's combatants to that of the enemy two panels ahead; in a link
-- battle it only dims (docs/engine/dimming-chip-effects.md §14).

local controller = require("./controller")

return define.chip {
    id = "lifesync",
    name = "LifeSync",
    description = "Makes HP\nsame as\nenemy",
    codes = { "*" },
    element = "null", family = "cursor", class = "standard",
    rarity = 1, mb = 12, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { "free_slot_in" },
    beast = { lockon = 4 },
    library = { number = 195, index = 195, sort = 254 },
    slot_in_limit = 3,
    dimming = controller.hook,
}
