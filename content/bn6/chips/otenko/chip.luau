--!strict
-- Otenko: a statue in front of the user that, for 1800 ticks, raises the
-- Atk+ bonus of the user's next damaging chip by 1 every 5 ticks, up to 50
-- (docs/engine/dimming-chip-effects.md §9).

local dimming_chips = require("../../lib/dimming")
local statue = require("./statue")

local WAIT = 0x3C
-- Where the statue goes (`byte_80E76C4`, both sides'): a solid panel with
-- nothing on it.
local SOLID, OCCUPIED = 0x10, 0x0F88_0080

-- `sub_80E7668`: the statue, then 60 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        local x, y = me.panel_x + me.facing, me.panel_y
        if field.check(x, y, SOLID, OCCUPIED) then
            local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
            statue.spawn(me.related1 :: Object, x, y, damage)
        end
        me.timer = WAIT
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        dimming_chips.done(me)
    end
end

local kind = define.kind {
    id = "otenko/controller",
    pool = "effect",
    -- `sub_80E7624`.
    update = dimming_chips.phases { name = "Otenko", effect = effect },
}

-- The chip's `dimming` hook (`sub_80E76D4`).
local function hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(kind, user, spec)
end

return define.chip {
    id = "otenko",
    name = "Otenko",
    description = "Put Otnko\nto raise\natk pwr",
    codes = { "O" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 4,
    mb = 66,
    damage = 1,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming" },
    extra_flags = { 1 },
    library = { number = 202, index = 2, sort = 324 },
    slot_in_limit = 1,
    dimming = hook,
}
