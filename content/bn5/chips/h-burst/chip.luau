--!strict
-- H-Burst (BN5 chip 0x152): the Program Advance of three Spreaders with consecutive codes: a shot along the row that
-- bursts eight times where it hits: the shared H-Burst (@common/h-burst, BN5's action 0x35 at 0x080EE35C) with BN5's
-- spreader (the attachment table's row 9, 0c-19: BN6's row 0x15), sound and shot (./burst). Its bursts: the shot's
-- init reads them from 0x080DA514 (5, 10, ...) at its parameters, the record's word 0x103, past the table's four
-- bytes into the next routine's code (0x080DA617): 8.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local action = require("@common/h-burst/action")
local attachment = require("../../objects/attachment/attachment")
local burst = require("./burst")
local recipe_energbom = require("../energbom/chip")
local recipe_megenbom = require("../megenbom/chip")
local recipe_spreader = require("../spreader/chip")

local LOOK: HyperBurstLook = {
    spreader = attachment.look { sprite = asset.sprite("bn5:spreader"), palette = 0, attach_point = 11 },
    attachment = attachment.kind,
    sound = asset.sound("bn5:h-burst"),
    burst = burst.kind,
}

return define.chip {
    id = "bn5:h-burst",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 50,
    hit_param = 5,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 18, sequence = { recipe_energbom, recipe_megenbom, recipe_spreader }, per_player_gauges_only = true },
        { order = 54, code_run = { chip = recipe_spreader, count = 3 } },
    },
    action = action.action({ id = "bn5:h-burst/action", bursts = 8 }, LOOK),
}
