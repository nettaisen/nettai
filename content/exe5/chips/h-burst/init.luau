--!strict
-- H-Burst (EXE5 chip 0x152): the Program Advance of three Spreaders with consecutive codes: a shot along the row that
-- bursts eight times where it hits: the shared H-Burst (@exelib/h_burst, EXE5's action 0x35 at 0x080EE35C) with EXE5's
-- spreader (the attachment table's row 9, 0c-19: EXE6's row 0x15), sound and shot (./burst). Its bursts: the shot's
-- init reads them from 0x080DA514 (5, 10, ...) at its parameters, the record's word 0x103, past the table's four
-- bytes into the next routine's code (0x080DA617): 8.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local action = require("@exelib/h_burst/action")
local attachment = require("../objects/attachment")
local burst = require("@self/burst")
local recipe_energbom = require("./energbom").energbom
local recipe_megenbom = require("./energbom").megenbom
local recipe_spreader = require("./spreader").spreader

local LOOK: HyperBurstLook = {
    spreader = attachment.look { sprite = asset.sprite("spreader"), palette = 0, attach_point = 11 },
    attachment = attachment.kind,
    sound = asset.sound("h-burst"),
    burst = burst.kind,
}

local chip: Chip = {
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
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 18, sequence = { recipe_energbom, recipe_megenbom, recipe_spreader }, operation_battle_only = true },
        { order = 54, code_run = { chip = recipe_spreader, count = 3 } },
    },
    action = action.action({ bursts = 8 }, LOOK),
}

return { ["h-burst"] = chip }
