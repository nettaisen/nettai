--!strict
-- GrabRvng and GrabBnsh: the grabbnsh series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local controller = require("@self/controller")

-- GrabRvng (EXE5 chip 0x076): the user's side takes back the panels the other side stole, a hand dropping on every
-- enemy navi for each: EXE6's controller (@exelib/grabbnsh, GrabBnsh's and GrabRvng's). EXE5's code is the same but
-- for the hand's effect number (0x29, EXE6's 0x3B), which is the same effect (the grab shot's animation 1).
local grabrvng: Chip = {
    codes = { "F", "N", "P" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 48,
    damage = 40,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook,
    auto_battle = { class = "when_area_lost" },
}

-- GrabBnsh (EXE5 chip 0x077): the user's side takes back the panels the other side stole, a hand dropping on every
-- enemy navi for each: EXE6's controller (@exelib/grabbnsh; EXE5's the same code).
local grabbnsh: Chip = {
    codes = { "D", "L", "R" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 24,
    damage = 20,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook,
    auto_battle = { class = "when_area_lost" },
}

return { grabrvng = grabrvng, grabbnsh = grabbnsh }
