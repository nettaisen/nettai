--!strict
-- PanlSht1 to PanlSht3: the dimming chips' variant 0x26, EXE6's leftover panel shot (EXE6's effect object #0x4F,
-- `sub_80E6254`, and attack object #0x7B, `sub_80D4A28`, which no EXE6 chip spawns) with EXE4's numbers. A controller
-- (effect object #0x4F, 0x080E79B8: the usual dimming phases, @exelib/dimming; its spawn 0x080E7A58 takes the user's
-- side alone) tears up the user's side's solid, free panels its chip picks (0x080E7A7A by the chip's first parameter;
-- each chip here its own), each shot forward after its wait (./panel) with the chip's damage and the telop's bonus;
-- once every panel is gone (0x080E7BA8), 30 ticks, then the undim.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local dimming_chips = require("@exelib/dimming")
local panel = require("@self/panel")
local panels = require("@exelib/panels")

type State = {
    -- The bonus the telop shows (the original's +0x32), added to the panels' damage.
    bonus: number,
    -- Its panels out (+0x60 on, a flag each, +0x68 their count).
    out: number,
}

local WAIT = 0x1E
-- The first panel's wait; each next one's 4 more.
local FIRST_WAIT, STAGGER = 40, 4
-- 0x080E7BE0 by the user's side: its own side's panel (the owner bit 0x20 set for side 1's, clear for side 0's),
-- solid, with nothing on it.
local SOLID, OCCUPIED, SIDE_1 = 0x10, 0x0F88_0080, 0x20

-- 0x080E7BC8: (x, y) is the user's side's, solid and free.
local function takes(me: Object, x: number, y: number): boolean
    local need, forbid = SOLID, OCCUPIED
    if me.alliance == 1 then
        need = bit32.bor(need, SIDE_1)
    else
        forbid = bit32.bor(forbid, SIDE_1)
    end
    return field.check(x, y, need, forbid)
end

-- Tear up (x, y) with the `n`th panel's wait.
local function tear(me: Object, x: number, y: number, n: number)
    local damage = dimming_chips.damage_plus(me, (me.state :: State).bonus)
    panel.spawn(me, x, y, damage, FIRST_WAIT + STAGGER * n)
end

-- 0x080E7A9C (PanlSht1's): the panel in front of the user.
local function front(me: Object)
    local x, y = me.panel_x + panels.side_direction(me.alliance), me.panel_y
    if takes(me, x, y) then
        tear(me, x, y, 0)
    end
end

-- 0x080E7AD2 (PanlSht2's): the user's row, from its side's far edge back to its own (0x080E7B38: column 6 by -1 for
-- side 0, 1 by +1 for side 1), while on the field.
local function row(me: Object)
    local x, step = 6, -1
    if me.alliance == 1 then
        x, step = 1, 1
    end
    local y = me.panel_y
    local n = 0
    while field.valid(x, y) do
        if takes(me, x, y) then
            tear(me, x, y, n)
            n += 1
        end
        x += step
    end
end

-- 0x080E7B3C (PanlSht3's): ahead of the user, above, below and behind (0x080E7B9C).
local function around(me: Object)
    local dir = panels.side_direction(me.alliance)
    local n = 0
    for _, d in { { 1, 0 }, { 0, -1 }, { 0, 1 }, { -1, 0 } } do
        local x, y = me.panel_x + dir * d[1], me.panel_y + d[2]
        if takes(me, x, y) then
            tear(me, x, y, n)
            n += 1
        end
    end
end

-- The controller of a chip whose panels `pick` tears up: the chip's `dimming` hook.
local function controller(id: string, pick: (me: Object) -> ()): (user: Object, spec: DimmingChipSpec) -> Object?
    -- 0x080E79FC by its phase.
    local function effect(me: Object)
        if me.phase == 0 then
            -- 0x080E7A18: the panels; once none is out, on.
            if me.phase_init == 0 then
                (me.state :: State).out = 0
                pick(me)
                me.phase_init = 4
            end
            if (me.state :: State).out == 0 then
                me.phase, me.phase_init = 4, 0
            end
        elseif me.phase == 4 then
            -- 0x080E7A36: 30 ticks, then the undim.
            if me.phase_init == 0 then
                me.timer = WAIT
                me.phase_init = 4
            end
            local t = me.timer - 1
            me.timer = int.u16(t)
            if t < 0 then
                dimming_chips.done(me)
            end
        else
            error(string.format("PanlSht's controller: phase %#x reads past its table (0x080E7A10)", me.phase))
        end
    end

    local kind = new.kind {
        id = id,
        pool = "effect",
        state = { bonus = "u16", out = "u8" },
        -- 0x080E79B8.
        update = dimming_chips.phases { name = "PanlSht", effect = effect },
    }

    -- 0x080E7A58.
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, spec, true)
        if o then
            (o.state :: State).bonus = spec.bonus
        end
        return o
    end
end

-- PanlSht1 (EXE4 chip 0x0B2): the panel in front.
local panlsht1: Chip = {
    codes = { "A", "H", "S", "*" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 1,
    mb = 19,
    damage = 100,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = controller("panlsht1/controller", front),
}

-- PanlSht2 (EXE4 chip 0x0B3): the user's row.
local panlsht2: Chip = {
    codes = { "C", "G", "T", "*" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 25,
    damage = 60,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = controller("panlsht2/controller", row),
}

-- PanlSht3 (EXE4 chip 0x0B4): the four panels around the user.
local panlsht3: Chip = {
    codes = { "F", "J", "N", "*" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 3,
    mb = 31,
    damage = 80,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = controller("panlsht3/controller", around),
}

return { panlsht1 = panlsht1, panlsht2 = panlsht2, panlsht3 = panlsht3 }
