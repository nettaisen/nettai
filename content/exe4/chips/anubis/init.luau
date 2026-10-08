--!strict
-- Anubis (EXE4 chip 0x0CC): the dimming chips' variant 0x18, a statue in front of the user that drains every enemy
-- navi 1 HP each 6 ticks for 3000 ticks, with bubbles over the enemy's area: EXE6's code (@exelib/anubis: its
-- controller, effect object #0x33, 0x080E5BF4; its statue, attack object #0x55, 0x080D82F8) in EXE4's look: sheet
-- 0c-30, its landing's 0xA2 and its breaking's 0x70 over effect row 0 (14-00); no NameID; the lifetime 3000 for either
-- variant (0x080D836A); its end reads no removal (0x080D83F8: it breaks however it goes) and releases no tracking;
-- EXE4's obstacle rows (0x080D83DC: row 3 does nothing, no frozen or bubbled); its drain row 0x31 at nothing; its
-- bubbles EXE4's (objects/rising_bubble).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local controller = require("@exelib/anubis/controller")
local rising_bubble = require("../objects/rising_bubble")
local statue = require("@exelib/anubis/statue")

-- 0x0801013A: back to its idle, no longer running while paused.
local function return_to_idle(me: Object)
    me.run_while_paused = false
    obstacle.return_to_idle(me)
end

local STATUE = statue.make {
    id = "anubis/statue",
    sprite = asset.sprite("anubis"),
    sounds = { land = asset.sound("metal-punch"), ["break"] = asset.sound("hit-bomb-1") },
    is = collision.own_body,
    hits = collision.hit_by_other_side,
    drain = collision.collision_31,
    nothing = collision.nothing,
    effects = {
        puff = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
        explosion = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 },
    },
    bubble = rising_bubble.kind,
    reads_no_removal = true,
    untracked = true,
    actions = {
        [1] = return_to_idle,
        [3] = function(_me: Object) end,
        [4] = obstacle.paralyzed,
        [5] = obstacle.knocked_back,
    },
}

-- Anubis's statue (0x080D82E8's row 0: animation 0, a drain every 6 ticks, 100 HP).
local VARIANT = statue.variant { anim = 0, period = 6, hp = 100, lifetime = 3000 }

local CONTROLLER = controller.make { id = "anubis/controller", statue = STATUE.spawn }

local chip: Chip = {
    codes = { "A" },
    element = "null",
    family = "summon",
    class = "mega",
    rarity = 4,
    mb = 86,
    damage = 1,
    hit_param = 1,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = CONTROLLER.hook(VARIANT),
}

return { anubis = chip }
