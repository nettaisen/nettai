--!strict
-- The machine swords' strike (the original's action 0x49, `sub_80EEB4C`):
-- the navi swings a blade, and on the swing's 10th tick every opposing navi
-- still in is slashed where it stands (a column of three panels) if the
-- sword finds it open: MchnSwrd a paralyzed one, ElemSwrd one on a grass,
-- ice or volcano panel, AssnSwrd either (`strike.action { ... }`).

local collision = require("../../rules/collision")
local parts = require("./parts")
local regions = require("../regions")

local strike = {}

export type State = {
    -- Ticks left in the phase (AIAttackVars+0x10).
    timer: number,
}

-- One state for every strike.
strike.STATE = { timer = "u16" }

local SWING, RECOVER = 0x0, 0x4

-- Grass, ice and volcano panels' flags.
local ELEMENT_PANELS = 0x1C00

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- A machine sword's strike.
function strike.action(spec: StrikeSpec): Action
    -- `sub_80EEC08`: slash `target` if the sword finds it open.
    local function slash(me: Object, target: Object)
        local x, y = target.panel_x, target.panel_y
        local open = (spec.finds ~= "on_element_panels" and target:status_timer("paralyze") ~= 0)
            or (spec.finds ~= "paralyzed" and bit32.btest(field.flags(x, y), ELEMENT_PANELS))
        if not open then
            return
        end
        local word = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = me.attack_element,
            region = regions.wide,
            -- None.
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.slash,
            damage = bit32.band(word, 0xFFFF),
            stamina = bit32.rshift(word, 16),
            hit_mod = 3,
        })
        -- The slash, flipped by the user's side (not its facing), in the
        -- sword's colours.
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, 0x10 * 0x10000), parts.effects.wide, me.alliance, spec.effect_palette)
    end

    -- `sub_80EEB68`: hold the blade and swing; the slashes land on the 10th
    -- tick, and the swing lasts 22.
    local function swing(me: Object, s: State)
        if me.step_init == 0 then
            me:open_counter_window()
            me:set_animation(5)
            battle.play_sound(parts.sounds.swing)
            parts.hold(me, spec.blade, 0)
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
        parts.drop(me)
        me:exit_attack()
    end

    -- `sub_80EEB4C`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == SWING then
            swing(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("action 0x49 phase %#x reads past its phase table", step))
        end
    end

    return define.action { state = strike.STATE, args = spec, update = update }
end

return strike
