--!strict
-- SignlRed (EXE4 chip 0x131, Blue Moon's): the dimming chips' variant 0x2B, a signal in front of the user that stops
-- the other side's chips while it is red: EXE4's own controller (effect object #0x58, 0x080E83F8: the usual dimming
-- phases, @exelib/dimming, and an effect, 0x080E843C, that places the signal on the panel ahead, by its side's way,
-- with the controller's damage word, and undims 30 ticks on) and its signal (./signal, attack object #0x81).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local dimming_chips = require("@exelib/dimming")
local signal = require("@self/signal")

-- The effect's ticks, counting its first.
local TICKS = 30

-- 0x080E843C: on its first tick the signal (0x080DD766) on the panel ahead (`object_getEnemyDirection`: by its
-- side); then the undim once the timer reaches 0.
local function effect(me: Object)
    if me.phase_init == 0 then
        local x = me.panel_x + (1 - 2 * me.alliance)
        signal.spawn(me, x, me.panel_y, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
        me.timer = TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t == 0 then
        dimming_chips.done(me)
    end
end

local controller = new.kind {
    id = "signlred/controller",
    pool = "effect",
    -- 0x080E83F8.
    update = dimming_chips.phases { name = "SignlRed", effect = effect },
}

local chip: Chip = {
    codes = { "S" },
    element = "null",
    family = "summon",
    class = "giga",
    rarity = 4,
    mb = 61,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    -- 0x080E8472: its controller (only the alliance byte of its user's).
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(controller, user, spec, true)
    end,
    auto_battle = { class = "landing_ahead" },
}

return { signlred = chip }
