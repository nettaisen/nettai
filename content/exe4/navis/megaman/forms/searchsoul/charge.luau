--!strict
-- SearchSoul's charged shot (`searchsoul/charged-shot`): weapon routine 0x0A (0x0800CDC4: 10 damage, counter byte 0x85,
-- every other attack variable 0) and the scope it starts (action 0x3E, 0x080EDB5C: EXE4's own; EXE5's action 0x3B grew
-- from it). MegaMan's scope finds the other side's navi (0x0800FB3C) and marks it (effect row 0x52, 20 pixels up);
-- five ticks on it fires five shots at its panel, six ticks apart, with the vulcan's sound: the first one through
-- invisibility (row 0x19), the rest not (row 0x0A), the last one flinching; then six ticks of recovery. With no navi
-- to find it goes straight to the recovery.

local collision = require("../../../../rules/collision")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")
local sparks = require("../../../../lib/sparks")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: ticks left in the current step.
    timer: number,
    -- AIAttackVars +0x12: shots left.
    shots: number,
    -- AIAttackVars +0x16 and +0x17: the panel it aims at.
    target_x: number,
    target_y: number,
}

-- The panels it looks in: those of the other side's navi, by side (0x0800FC38).
local OTHER_NAVI = { [0] = 0x0020_0000, [1] = 0x0040_0000 }
-- The scope's mark (effect row 0x52: 10-23), 20 pixels up.
local MARK = new.effect { sprite = asset.sprite("scope-mark"), anim = 0, palette = 0 }
local MARK_Z = 20 * 0x1_0000
local SHOT_SOUND = asset.sound("vulcan")
local ANIM = 10
local SHOTS, AIM_TICKS, SHOT_TICKS, RECOVER_TICKS = 5, 4, 5, 5
-- A shot's hit modifier: the last one's flinches (3), the others' don't (1).
local HIT_MOD, LAST_HIT_MOD = 1, 3

-- 0x0800FB3C: the panel of the other side's navi nearest ahead of `me` by its side alone (side 0's ahead is its own
-- column and those to the right; side 1's its own column and those to the left), else nearest behind, by column
-- distance; of those as near as the nearest so far, the later one in the list; nil for none.
local function find(me: Object): (number?, number?)
    local list = panels.filtered_except(me, OTHER_NAVI[me.alliance], 0)
    local function pick(ahead: boolean): { number }
        local out: { number } = {}
        for _, p in list do
            local x = panels.x(p)
            local takes
            if me.alliance == 0 then
                takes = if ahead then x >= me.panel_x else x < me.panel_x
            else
                takes = if ahead then x <= me.panel_x else x > me.panel_x
            end
            if takes then
                out[#out + 1] = p
            end
        end
        return out
    end
    local found = pick(true)
    if #found == 0 then
        found = pick(false)
        if #found == 0 then
            return nil, nil
        end
    end
    local bx, by = panels.x(found[1]), panels.y(found[1])
    local best = math.abs(bx - me.panel_x)
    for i = 2, #found do
        local x, y = panels.x(found[i]), panels.y(found[i])
        local dx = math.abs(me.panel_x - x)
        -- (A tie compares the rows and takes the panel whatever they are.)
        if dx <= best then
            bx, by, best = x, y, dx
        end
    end
    return bx, by
end

-- 0x080EDB80: the target found and marked; four ticks on, the shots. No target: the recovery.
local function aim(me: Object, s: State)
    if me.step_init ~= 0 then
        local t = s.timer - 1
        s.timer = bit32.band(t, 0xFFFF)
        if t < 0 then
            me.step, me.step_init = 4, 0
        end
        return
    end
    me:set_status("using_action", true)
    s.shots = SHOTS
    local x, y = find(me)
    if x == nil or y == nil then
        me.step, me.step_init = 8, 0
        return
    end
    s.target_x, s.target_y = x, y
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, MARK_Z), MARK)
    s.timer = AIM_TICKS
    me.step_init = 4
end

-- 0x080EDBD0: a shot at the target's panel (the counter window open; the hitbox's parameters 0x19050001 for the
-- first, 0x0A050001 after: one panel, spark 0, at navis), six ticks; after the fifth, the recovery.
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(ANIM)
        me:open_counter_window()
        s.timer = SHOT_TICKS
        battle.hitbox(me, {
            panel_x = s.target_x,
            panel_y = s.target_y,
            element = me.attack_element,
            region = regions.single,
            hit_effect = sparks.plain,
            target = collision.hits_navis,
            self_type = if s.shots == SHOTS then collision.curse else collision.thrown,
            damage = me.attack_damage,
            stamina = me.hit_param,
            hit_mod = if s.shots > 1 then HIT_MOD else LAST_HIT_MOD,
        })
        battle.play_sound(SHOT_SOUND)
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        me.step_init = 0
        s.shots -= 1
        if s.shots <= 0 then
            me.step, me.step_init = 8, 0
        end
    end
end

-- 0x080EDC3C: six ticks; then back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        me:exit_attack()
    end
end

charge.action = new.action {
    id = "searchsoul/charged-shot/action",
    state = { timer = "u16", shots = "u16", target_x = "u8", target_y = "u8" },
    -- 0x080EDB5C
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0 then
            aim(me, s)
        elseif step == 4 then
            shoot(me, s)
        elseif step == 8 then
            recover(me, s)
        else
            error(string.format("SearchSoul's charged shot's step %#x reads past its table (0x080EDB74)", step))
        end
    end,
}

-- 0x0800CDC4.
local function setup(navi: Object): Action
    navi.attack_damage = 10
    navi.attack_element = 0
    navi.attack_variant = 0
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = 0x85
    return charge.action
end

charge.weapon = new.weapon {
    id = "searchsoul/charged-shot",
    -- The charge table's row 0x0A (0x080180B8 by AIData +0x0F), by Charge 0 to 4; Charge 5 reads on into row 0x0B's
    -- first entry.
    charge_ticks = { 100, 100, 100, 100, 100, 40 },
    setup = setup,
}

return charge
