--!strict
-- SearchSoul's charged shot (`searchsoul/charge`): the original's weapon routine 8 (0x0800F696) sets 10 damage and 2
-- more per point of buster Attack plus one (0x0800FE5E), its own counter byte and no element, and starts action 0x3B
-- (0x080EE854): MegaMan's scope finds the enemy navi nearest ahead (0x08012E50), marks it (effect 0x68, 20 pixels
-- up) and five ticks on fires five shots at its panel, six ticks apart, the last one flinching; then six ticks of
-- recovery. With no enemy navi to find it goes straight to the recovery.

local collision = require("../../../../rules/collision")
local panels = require("@common/panels")
local regions = require("@common/regions")
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

local HIT_PARAM = 0x8A
-- Panels holding the other side's navi, by side (0x08012F58).
local ENEMY_NAVI = { [0] = 0x0020_0000, [1] = 0x0040_0000 }
-- The scope's mark (effect 0x68: sprite 10-23), 20 pixels up.
local MARK = define.effect { sprite = asset.sprite("bn5:search-scope"), anim = 0, palette = 0 }
local MARK_Z = 20 * 0x1_0000
local SOUND_MARK, SOUND_SHOT = asset.sound("bn5:mark"), asset.sound("bn5:col-army-2")
local ANIM = 17
local SHOTS, AIM_TICKS, SHOT_TICKS, RECOVER_TICKS = 5, 4, 5, 5
-- A shot's hit modifier: the last one's flinches (3), the others' don't (1).
local HIT_MOD, LAST_HIT_MOD = 1, 3

-- 0x08012E50: the panel of the other side's navi nearest ahead of `me` (none ahead: nearest behind), by column
-- distance; of those as near as the nearest so far, the later one in the list (rows 3 to 1, columns 6 to 1); nil
-- for none.
local function find(me: Object): (number?, number?)
    local list = panels.filtered_except(me, ENEMY_NAVI[me.alliance], 0)
    local right = bit32.bxor(me.alliance, me.flip) == 0
    local function pick(ahead: boolean): { number }
        local out = {}
        for _, p in list do
            local x = panels.x(p)
            local takes = if ahead == right then x > me.panel_x else x < me.panel_x
            if takes then
                table.insert(out, p)
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

-- 0x080EE874: the target found and marked; four ticks on, the shots. No target: the recovery.
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
    battle.play_sound(SOUND_MARK)
    s.timer = AIM_TICKS
    me.step_init = 4
end

-- 0x080EE8CA: a shot at the target's panel (the counter window open), six ticks; after the fifth, the recovery.
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(ANIM)
        me:open_counter_window()
        s.timer = SHOT_TICKS
        battle.hitbox(me, {
            panel_x = s.target_x,
            panel_y = s.target_y,
            element = me.attack_element,
            z = 0,
            region = regions.single,
            hit_effect = sparks.plain,
            target = collision.hits_navis,
            self_type = collision.curse,
            damage = me.attack_damage,
            stamina = me.hit_param,
            hit_mod = if s.shots > 1 then HIT_MOD else LAST_HIT_MOD,
        })
        battle.play_sound(SOUND_SHOT)
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

-- 0x080EE934: six ticks; then back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        me:set_animation(0)
        me:exit_attack()
    end
end

charge.action = define.action {
    id = "bn5:searchsoul/charge/action",
    state = { timer = "u16", shots = "u16", target_x = "u8", target_y = "u8" },
    -- 0x080EE854
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0 then
            aim(me, s)
        elseif step == 4 then
            shoot(me, s)
        elseif step == 8 then
            recover(me, s)
        else
            error(string.format("SearchSoul's charged shot's step %#x reads past its table (0x080EE868)", step))
        end
    end,
}

-- 0x0800F696
local function setup(navi: Object): Action
    navi.attack_damage = 10 + 2 * (battle.navi(navi.alliance).attack + 1)
    navi.hit_param = HIT_PARAM
    navi.attack_element = 0
    return charge.action
end

charge.weapon = define.weapon {
    id = "bn5:searchsoul/charge",
    -- The charge table's row 8 (0x0801CABC); Charge 5 reads on into row 9's first entry.
    charge_ticks = { 110, 105, 100, 95, 90, 80 },
    setup = setup,
}

return charge
