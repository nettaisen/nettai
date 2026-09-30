--!strict
-- MchnSwrd, ElemSwrd and AssnSwrd: action 0x49 (`sub_80EEB4C`).
--
-- The navi swings a blade, and on the swing's 10th tick every opposing
-- navi still in is slashed where it stands (a column of three panels),
-- if MchnSwrd finds it paralyzed, ElemSwrd finds it on a grass, ice or
-- volcano panel, and AssnSwrd finds either. The variant (the chip's
-- subtype) picks which; the blade is the chip's data
-- (`data.chips[id].sword`). The navi's own request 0x80000 (which nothing
-- in the game sets) also starts this action from idle.

local sword = require("../../lib/sword")

export type State = {
    -- Ticks left in the phase (AIAttackVars+0x10).
    timer: number,
}

local strike = {
    state = { timer = "u16" },
}

local SWING, RECOVER = 0x0, 0x4

-- The variants: paralyzed targets only, targets on those panels only (the
-- third takes both).
local PARALYZED_ONLY, PANELS_ONLY = 0, 1
-- Grass, ice and volcano panels' flags.
local ELEMENT_PANELS = 0x1C00

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- `sub_80EEC08`: slash `target` if the variant finds it open.
local function slash(me: Object, target: Object)
    local v = me.variant
    local x, y = target.panel_x, target.panel_y
    local open = (v ~= PANELS_ONLY and target:status_timer("paralyze") ~= 0)
        or (v ~= PARALYZED_ONLY and bit32.btest(field.flags(x, y), ELEMENT_PANELS))
    if not open then
        return
    end
    local word = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.attack_element,
        region = 0x04,
        hit_effect = 0xFF,
        target = 0x05,
        self_type = 0x07,
        damage = bit32.band(word, 0xFFFF),
        stamina = bit32.rshift(word, 16),
        hit_mod = 3,
    })
    -- The slash, flipped by the user's side (not its facing), in the
    -- variant's palette.
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0x10 * 0x10000), 0x16, me.alliance, v + 7)
end

-- `sub_80EEB68`: hold the blade and swing; the slashes land on the 10th
-- tick, and the swing lasts 22.
local function swing(me: Object, s: State)
    if me.step_init == 0 then
        me:open_counter_window()
        me:set_animation(5)
        battle.play_sound(0xB0)
        sword.hold_blade(me, 0)
        me:set_status("using_action", true)
        s.timer = 0x15
        me.step_init = 4
    end
    if s.timer == 0x0C then
        for _, target in battle.alive_actors(bit32.bxor(me.alliance, 1)) do
            slash(me, target)
        end
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        set_step(me, RECOVER)
    end
end

-- `sub_80EEBE2`: 6 ticks, then back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = 5
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    sword.drop_blade(me)
    me:exit_attack()
end

-- `sub_80EEB4C`.
function strike.update(me: Object, s: State)
    local step = me.step
    if step == SWING then
        swing(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("action 0x49 phase %#x reads past its phase table", step))
    end
end

return strike
