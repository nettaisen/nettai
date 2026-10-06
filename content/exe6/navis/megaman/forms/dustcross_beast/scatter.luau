--!strict
-- DustCross Beast's charged Null chip (`dustcross-beast/scatter`): the
-- original's weapon routine 0x1D (`sub_8011DFC`) and action 0x50
-- (`sub_80EF328`). The navi rises (its body's hit off, its panel kept
-- reserved), picks up to six panels of the 3x3 block two panels ahead (the
-- enemies' first, in random order), throws junk at them (./junk_shot)
-- every 11 ticks from high up, and lands. While it runs the rules
-- doesn't ground the navi (the role `dust_beast_scatter`).

local junk_shot = require("./junk_shot")
local regions = require("@exelib/regions")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- AIAttackVars+0x10: ticks of the phase. (The throws to go are the
    -- navi's `attack_count`.)
    timer: number,
    -- AIAttackVars+0x30..+0x35: the target panels (x | y << 4; 0 none),
    -- thrown at from the last.
    targets: { number },
}

local scatter = {}

-- The phases, on the attack's step.
local RISE, THROW, LAND = 0x0, 0x4, 0x8

-- Break (secondary element 0x10), the hit's counter byte.
local BREAK, HIT_PARAM = 0x10, 0x9E

-- The rise: its first speed and acceleration, 16.16.
local RISE_SPEED, RISE_ACCELERATION = 0x6000, 0x3000

-- The panels it picks from: the 3x3 block around the panel two ahead,
-- facing right whatever the navi's side (`sub_80EF3F4` passes side 0),
-- with this panel bit set; bodies on them come first.
local BLOCK = regions.square
local PICKABLE, BODY = 0x1_0000, 0x0C00_0000

-- Throws, and the shuffles' swaps (`sub_8000C72`).
local THROWS, SWAPS = 6, 10

-- `sub_8011DFC`: 80 damage and 20 more per point of buster Attack (up to
-- 5), Break.
local function setup(navi: Object): Action
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_damage = weapon.charge_damage(navi, 0x50, 0x14)
    navi.attack_element = BREAK
    return scatter.action
end

-- `sub_8000C72`: `swaps` times, swap two of the `n` entries of `list` from
-- `first` on, each picked with a draw.
local function shuffle(list: { number }, first: number, n: number, swaps: number)
    if n == 0 then
        error("shuffling no panels divides by zero (sub_8000C72; the BIOS hangs)")
    end
    for _ = 1, swaps do
        local i = first + battle.rng_positive() % n
        local j = first + battle.rng_positive() % n
        list[i], list[j] = list[j], list[i]
    end
end

-- `sub_80EF3F4`: the target panels.
local function pick_targets(me: Object, s: State)
    local cx, cy = me.panel_x + me.facing * 2, me.panel_y
    -- object_getPanelRegion: the block's panels with the bit, in order.
    local found = { 0, 0, 0, 0, 0, 0, 0, 0 }
    local count = 0
    for _, offset in BLOCK.panels :: { { number } } do
        local x, y = cx + offset[1], cy + offset[2]
        if field.check(x, y, PICKABLE, 0) then
            count += 1
            found[count] = int.u8(x + y * 0x10)
        end
    end
    if count ~= THROWS then
        if count == 0 then
            error("DustCross Beast's scatter finds no panel and runs through memory (sub_80EF3F4)")
        end
        -- The panels with a body first; the rest shuffled.
        local bodies = 0
        for i = 1, count do
            local p = found[i]
            if bit32.btest(field.flags(bit32.band(p, 7), bit32.rshift(p, 4)), BODY) then
                bodies += 1
                found[i], found[bodies] = found[bodies], found[i]
            end
        end
        if bodies ~= 0 then
            shuffle(found, bodies + 1, count - bodies, SWAPS)
        end
    end
    shuffle(found, 1, THROWS, SWAPS)
    for i = 1, THROWS do
        s.targets[i] = found[i]
    end
    -- (The game copies them to the attack's +0x30, the marker word's
    -- place.)
    me.marker = found[1] + found[2] * 0x100 + found[3] * 0x1_0000 + found[4] * 0x100_0000
end

-- `sub_80EF348`: rise for 17 ticks.
local function rise(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x11)
        me:open_counter_window()
        me:set_status("using_action", true)
        me:set_status("moving", true)
        local v = me.vel
        me.vel = Vec3.new(v.x, v.y, RISE_SPEED)
        s.timer = 0x10
        me.collision:set_region(nil)
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        me:reserve_panel(me.panel_x, me.panel_y)
        me.step_init = 4
    end
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(p.x, p.y, p.z + v.z)
    me.vel = Vec3.new(v.x, v.y, v.z + RISE_ACCELERATION)
    local left = s.timer - 1
    s.timer = left
    if left >= 0 then
        return
    end
    me.step = THROW
    me.step_init = 0
end

-- `sub_80EF3A0`: the targets; then a throw every 11 ticks, from the last.
local function throw(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 1
        pick_targets(me, s)
        s.timer = 4
        me.attack_count = THROWS
        me:set_animation(0x19)
    end
    local left = s.timer - 1
    s.timer = left
    if left >= 0 then
        return
    end
    s.timer = 0xA
    local t = s.targets[me.attack_count]
    if t ~= 0 then
        local damage = bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16))
        junk_shot.spawn(me, me.panel_x, me.panel_y, me.attack_element, damage, me.pos.z, t)
    end
    local throws = me.attack_count - 1
    me.attack_count = throws
    if throws ~= 0 then
        return
    end
    s.timer = 0x1E
    me.step = LAND
    me.step_init = 0
end

-- `sub_80EF47A`: 31 ticks, then down on its panel, and idle.
local function land(me: Object, s: State)
    local left = s.timer - 1
    s.timer = left
    if left >= 0 then
        return
    end
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me.collision:set_region(regions.single)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    me:set_status("moving", false)
    me:exit_attack()
end

scatter.action = define.action {
    id = "dustcross-beast/scatter/action",
    state = { timer = "u16", targets = "u8[6]" },
    -- `sub_80EF328`.
    update = function(me: Object, s: State)
        local step = me.step
        if step == RISE then
            rise(me, s)
        elseif step == THROW then
            throw(me, s)
        elseif step == LAND then
            land(me, s)
        else
            error(string.format("DustCross Beast's scatter phase %#x reads past its phase table (sub_80EF328)", step))
        end
    end,
}

scatter.weapon = define.weapon {
    id = "dustcross-beast/scatter",
    -- The charge table's row 0x1D, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 40, 40, 40, 40, 40, 20, 20, 20 },
    setup = setup,
}

return scatter
