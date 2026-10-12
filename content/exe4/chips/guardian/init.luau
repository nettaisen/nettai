--!strict
-- Guardian (EXE4 chip 0x0CB): the dimming chips' variant 0x28, a statue in front of the user that strikes back at the
-- navi whose attack breaks it: EXE6's code (@exelib/guardian: its controller, effect object #0x52, 0x080E7ED0; its
-- statue, attack object #0x7D, 0x080DCDA8; its strike's dimming, effect object #0x53, 0x080E7F84, naming Punisher,
-- ../punisher) in EXE4's look and its older ways: sheet 0c-35, 10 HP, no NameID; its arrival's flash (effect row 0x16)
-- and 0xA0 as it stands; its strike row 0x17 at navis on their panels (0x080DCFDC), lightning (row 0x61) over them and
-- 0x143; its puffs row 0x15; it crumbles without a sound (0x080DD052); its end reads no removal and releases no
-- tracking (0x080DD024), any hit flags send it on to strike back, the side picked there (0x080DCF00); striking, it keeps
-- its collision on its panel; EXE4's obstacle rows (0x080DCE78).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local controller = require("@exelib/guardian/controller")
local punisher = require("./punisher").punisher
local rising_bubble = require("../objects/rising_bubble")
local statue = require("@exelib/guardian/statue")
local strike = require("@exelib/guardian/strike")

-- 0x0801013A: back to its idle, no longer running while paused.
local function return_to_idle(me: Object)
    me.run_while_paused = false
    obstacle.return_to_idle(me)
end

local STATUE = statue.make {
    id = "guardian/statue",
    sprite = asset.sprite("guardian-statue"),
    hp = 10,
    sounds = { place = asset.sound("tops"), strike = asset.sound("lightning") },
    arrival = new.effect { sprite = asset.sprite("puff"), anim = 3, palette = 0 },
    burst = new.effect { sprite = asset.sprite("lightning"), anim = 0, palette = 0 },
    is = collision.neutral_body,
    hits = collision.everything,
    strike_is = collision.thrown_piercing_break,
    strike_hits = collision.hits_navis,
    strike = strike.make { id = "guardian/strike", punisher = punisher },
    effects = { puff = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 } },
    bubble = rising_bubble.kind,
    reads_no_removal = true,
    untracked = true,
    strikes_on_any_hit = true,
    keeps_region_striking = true,
    actions = {
        [1] = return_to_idle,
        [3] = function(_me: Object) end,
        [4] = obstacle.paralyzed,
        [5] = obstacle.knocked_back,
    },
}

local CONTROLLER = controller.make { id = "guardian/controller", statue = STATUE.spawn }

local chip: Chip = {
    codes = { "O" },
    element = "null",
    family = "summon",
    class = "mega",
    rarity = 3,
    mb = 64,
    damage = 200,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = CONTROLLER.hook,
}

return { guardian = chip }
