--!strict
-- MetlGer1 to MetlGer3: the dimming chips' variant 0x22, EXE4's own. A controller (effect object #0x49, 0x080E7244:
-- the usual dimming phases, @exelib/dimming; its spawn 0x080E72EC takes the user's side, not its facing) whose effect
-- (0x080E7288) sets a gear (objects/gear, its mode 1: rolling for 1800 ticks on the other side, one of its side's
-- field objects) on the other side's solid, free panel of the user's row nearest that side's far edge, with the
-- chip's damage (no telop bonus) and a sound (0xA0); then 60 ticks. No such panel, no gear.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local dimming_chips = require("@exelib/dimming")
local gear = require("../objects/gear")
local panels = require("@exelib/panels")

local SOUND = asset.sound("tops")
local WAIT = 60
-- The gear's mode (the controller's first parameter): a lifetime gear.
local MODE = 1
-- The gear's panel: solid, without a body, of the other side (0x080E72DC by the user's side: the owner bit 0x20 set
-- for side 1's panels, clear for side 0's).
local SOLID, BODIES, SIDE_1 = 0x10, 0x0F80_0000, 0x20

-- 0x080E7288.
local function effect(me: Object)
    if me.phase_init == 0 then
        local other = bit32.bxor(me.alliance, 1)
        local need, forbid = SOLID, BODIES
        if other == 1 then
            need = bit32.bor(need, SIDE_1)
        else
            forbid = bit32.bor(forbid, SIDE_1)
        end
        local x = panels.closest_in_row(other, me.panel_y, need, forbid)
        if x ~= 0 then
            local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
            gear.spawn(x, me.panel_y, me.element, me.alliance, MODE, damage)
            battle.play_sound(SOUND)
        end
        me.timer = WAIT
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    dimming_chips.done(me)
end

local kind = new.kind {
    id = "metlger/controller",
    pool = "effect",
    -- 0x080E7244.
    update = dimming_chips.phases { name = "MetlGer", effect = effect },
}

-- 0x080E72EC.
local function hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(kind, user, spec, true)
end

-- MetlGer1 (EXE4 chip 0x0AF).
local metlger1: Chip = {
    codes = { "A", "M", "P", "*" },
    element = "null",
    family = "metal",
    class = "standard",
    rarity = 1,
    mb = 20,
    damage = 100,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = hook,
}

-- MetlGer2 (EXE4 chip 0x0B0).
local metlger2: Chip = {
    codes = { "C", "E", "G", "*" },
    element = "null",
    family = "metal",
    class = "standard",
    rarity = 2,
    mb = 28,
    damage = 130,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = hook,
}

-- MetlGer3 (EXE4 chip 0x0B1).
local metlger3: Chip = {
    codes = { "I", "L", "Q", "*" },
    element = "null",
    family = "metal",
    class = "standard",
    rarity = 3,
    mb = 36,
    damage = 160,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = hook,
}

return { metlger1 = metlger1, metlger2 = metlger2, metlger3 = metlger3 }
