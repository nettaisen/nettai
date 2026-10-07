--!strict
-- DrkStage (EXE4 chip 0x0C1): a dark chip, which no folder holds and the custom screen offers (rules/dark_chips). A
-- dimming chip (action 0x0C, its variant 59: the controller 0x080E98BC spawns, effect object #0x6A): after the dim and
-- the telop its effect (0x080E987C) hands two panel changes to two panel changers on its panel (objects/panel_changer)
-- and waits until one is done: the user's own area turns to holes (0x080E423C's row 12, no flicker) and the other
-- side's to poison (row 13), each over 30 ticks. Its cost: one chip fewer on the custom screen (rules/dark_chips).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local dimming_chips = require("@exelib/dimming")
local panel_changer = require("../objects/panel_changer")

-- Row 12: the own area to holes.
local HOLES = panel_changer.change {
    panel = "hole",
    panels = "all",
    flicker = false,
    ticks = 30,
    sides = panel_changer.OWN_AREA,
    start_sound = panel_changer.START_SOUND,
    tick_sound = panel_changer.TICK_SOUND,
    change_sound = panel_changer.CHANGE_SOUND,
}

-- Row 13: the other side's area to poison.
local POISON = panel_changer.change {
    panel = "poison",
    panels = "all",
    flicker = true,
    ticks = 30,
    sides = panel_changer.OTHER_AREA,
    tick_sound = panel_changer.TICK_SOUND,
    change_sound = panel_changer.POISON_SOUND,
}

export type State = {
    -- The changers run (the original's Param2, whose address each holds).
    busy: boolean,
}

-- 0x080E987C: the two changers, then wait.
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        s.busy = true
        panel_changer.spawn(me, me.panel_x, me.panel_y, HOLES, me)
        s.busy = true
        panel_changer.spawn(me, me.panel_x, me.panel_y, POISON, me)
        me.phase_init = 4
    end
    if not s.busy then
        dimming_chips.done(me)
    end
end

-- 0x080E9838.
local controller = new.kind {
    id = "drkstage/controller",
    pool = "effect",
    state = { busy = "bool" },
    update = dimming_chips.phases { name = "DrkStage", effect = effect },
}

local drkstage: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "dark" },
    -- 0x080E98BC: the controller on the user's panel and side (no facing of the user's).
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(controller, user, spec, true)
    end,
}

return { drkstage = drkstage }
