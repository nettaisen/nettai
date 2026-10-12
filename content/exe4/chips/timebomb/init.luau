--!strict
-- TimeBomb (EXE4 chip 0x061): the dimming chips' variant 0x11 (./bomb): a countdown bomb on the other side's front
-- column that counts 3, 2, 1 and blows up that side's area unless it is broken first.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local bomb = require("@self/bomb")
local countdown = require("@exelib/timebom/countdown")

-- TimeBomb's bomb (its first parameter 0: animations 0 and 1, 50 HP).
local BOMB = countdown.variant { appear_anim = 0, idle_anim = 1, hp = 50, class = 1, collision = bomb.body }

-- TimeBomb (EXE4 chip 0x061).
local timebomb: Chip = {
    codes = { "J", "K", "L" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 55,
    damage = 150,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = bomb.controller.hook(BOMB),
    auto_battle = { class = "when_far_front_free" },
}

return { timebomb = timebomb }
