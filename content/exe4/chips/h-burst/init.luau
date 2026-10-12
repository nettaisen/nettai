--!strict
-- H-Burst (EXE4 chip 0x15D): the Program Advance of three Spreaders of consecutive codes (the recipe table, the
-- pointer list at 0x080212B0): a shot along the row that bursts five times where it hits. The shared H-Burst
-- (@exelib/h_burst: EXE4's action 0x71, 0x080EE93A, and its shot, attack object #0x9E, 0x080E0B46) with EXE4's look:
-- the navi aims in its animation 14, holding the Spreader's gun (attachment row 0x0A) as the spreaders hold theirs
-- (./spreader/spreader), with the spreaders' sound (0x8D); the shot's probe of no type (row 0), its bursts an attack
-- hitting navis with no status and hit modifier 0 (0x080E0C4C), their explosions effect row 0x68 (0x080E1698: 14-04,
-- animation 0 in palette 2). Its bursts: the shot's init reads them from 0x080E0B44 (5, 10) at its parameter word,
-- the chip's, 0: 5.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local action = require("@exelib/h_burst/action")
local attachment = require("../objects/attachment")
local burst = require("@exelib/h_burst/burst")
local collision = require("../rules/collision")
local spreader = require("./spreader/spreader")
local spreaders = require("./spreader")

local SHOT = burst.make_kind("h-burst/burst", {
    probe = collision.nothing,
    is = collision.attack,
    hits = collision.hits_navis,
    explosion = new.effect { sprite = asset.sprite("pink-flash"), anim = 0, palette = 2 },
})

local LOOK: HyperBurstLook = {
    spreader = attachment.look { sprite = asset.sprite("spreader"), palette = 0 },
    hold = spreader.hold,
    slot = "related",
    sound = asset.sound("spreader"),
    burst = SHOT,
    anim = 14,
}

local chip: Chip = {
    action = action.action({ id = "h-burst/action", bursts = 5 }, LOOK),
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 100,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 36, code_run = { chip = spreaders.spreader, count = 3 } } },
    auto_battle = { class = "behind_target" },
}

return { ["h-burst"] = chip }
