--!strict
-- EnergBom and MegEnBom: the energbom series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local bombs = require("../lib/bombs")
local burst = require("@self/burst")
local collision = require("../rules/collision")

-- EnergBom (EXE5 chip 0x01e): a bomb that leaves an energy burst where it lands (three hits on that panel): EXE6's
-- throw, bomb and burst (@exelib/bombs, @exelib/energbom/burst; EXE5's the same code), with EXE5's collision
-- types.
local energbom: Chip = {
    codes = { "E", "P", "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 11,
    damage = 40,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = bombs.throw {
        held = bombs.held,
        held_anim = 2,
        thrower = bombs.thrower(bombs.variant {
            collision = { is = collision.thrown, hits = collision.hits_navis },
            hit_mod = 1,
            palette = 0,
            anim = 3,
            after = burst.leave,
        }),
    },
    auto_battle = { class = "into_reach", range = "three_ahead" },
}

-- MegEnBom (EXE5 chip 0x01f): a bomb that leaves an energy burst where it lands (three hits on that panel): EXE6's
-- throw, bomb and burst (@exelib/bombs, @exelib/energbom/burst; EXE5's the same code), with EXE5's collision
-- types.
local megenbom: Chip = {
    codes = { "C", "K", "Z" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 60,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = bombs.throw {
        held = bombs.held,
        held_anim = 2,
        thrower = bombs.thrower(bombs.variant {
            collision = { is = collision.thrown, hits = collision.hits_navis },
            hit_mod = 1,
            palette = 0,
            anim = 3,
            after = burst.leave,
        }),
    },
    auto_battle = { class = "into_reach", range = "three_ahead" },
}

return { energbom = energbom, megenbom = megenbom }
