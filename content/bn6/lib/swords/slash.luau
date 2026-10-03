--!strict
-- The swords' slash (the original's action 0x13, `sub_80EB776`): the navi
-- holds a blade and slashes the panels ahead, one hit on the 10th tick of
-- the swing, and an effect that draws the slash. Each chip composes its own
-- (`slash.action { blade = ..., hit = ..., effect = ..., sound = ... }`,
-- docs/design/content-model-v2.md §5.2): DblDream swings twice, CrosSwrd
-- adds a single-panel hit, the elemental swords color the slash.
--
-- A step sword (StepSwrd) first steps two panels ahead, if that panel is on
-- the field with nobody on it, leaving afterimages, and steps back after
-- the swing (not in a Beast form). With nowhere to step it doesn't swing.

local collision = require("../../rules/collision")
local parts = require("./parts")
local regions = require("@common/regions")

local slash = {}

export type State = {
    -- Ticks left in the phase (AIAttackVars+0x10). (The swings left are
    -- the navi's `attack_count`.)
    timer: number,
    -- The navi turned round while stepping (+0x34; nothing in the
    -- player's actions sets it).
    turned: number,
}

-- One state for every sword's slash.
slash.STATE = { timer = "u16", turned = "u32" }

-- The phases, on the attack's step.
local START, SET_UP, SWING, STEP_BACK, RECOVER = 0x0, 0x4, 0x8, 0xC, 0x10

-- The panel a step sword steps to: on the field (every panel type's flag
-- 0x10000), with no body on it.
local PANEL_EXISTS, BODIES = 0x10000, 0x0F800000

-- The afterimages' color shader.
local AFTERIMAGE_SHADER = 0xFFE0

-- CrosSwrd's second hit: a slash on the panel ahead alone.
slash.CROSS_HIT = {
    region = regions.single,
    collision = { is = collision.slash, hits = collision.hits_navis },
    hit_mod = 3,
}

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- `object_getFlip`.
local function flip(me: Object): number
    return bit32.bxor(me.alliance, me.flip)
end

-- The flip that ends a step sword's step turned round (`sub_800F2C6`: the
-- sprite, and the HP number's place, which the engine doesn't draw).
local function turn(me: Object)
    me.flip = bit32.bxor(me.flip, 1)
    me.sprite:set_flip(flip(me))
end

-- The damage word: the chip's damage and hit parameter, plus the bonus.
local function damage_word(me: Object): number
    return int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
end

-- A chip's slash.
function slash.action(spec: SlashSpec): Action
    -- The blade's animation (`sub_80EBAE8`, or the spec's game's own).
    local blade_anim = spec.blade_anim or parts.blade_anim

    -- `sub_80EB79C`: straight to the swing, or for a step sword, the step.
    local function start(me: Object, s: State)
        set_step(me, SET_UP)
        if not spec.step then
            return
        end
        if me.special_source == 0 and not battle.navi(me.alliance).beast then
            -- sub_800E5FC: not while moving or sliding.
            if me:status("moving") or me:status("sliding") then
                return
            end
            me:set_status("moving", true)
            me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
            me:reserve_panel(me.panel_x, me.panel_y)
        end
        s.turned = 0
        -- sub_8015B00: two panels ahead.
        local x, y = me.panel_x + 2 * me.facing, me.panel_y
        if not field.check(x, y, PANEL_EXISTS, BODIES) then
            s.turned = 0
            me:set_animation(4)
            set_step(me, STEP_BACK)
            return
        end
        battle.afterimage(me, me.pos, {
            anim = 0,
            flip = flip(me),
            color_shader = AFTERIMAGE_SHADER,
            lifetime = 0x14,
            shadow = "ground",
        })
        me.panel_x, me.panel_y = x, y
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        if s.turned == 1 then
            turn(me)
        end
    end

    -- `sub_80EB84C`: one swing, or DblDream's two.
    local function set_up(me: Object, s: State)
        me.attack_count = spec.swings or 1
        me.marker = 0
        set_step(me, SWING)
    end

    -- A hit on the panel ahead.
    local function hit(me: Object, x: number, y: number, h: SwordHit, word: number)
        battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = me.attack_element,
            region = h.region,
            -- None.
            hit_effect = nil,
            target = h.collision.hits,
            self_type = h.collision.is,
            damage = bit32.band(word, 0xFFFF),
            stamina = bit32.rshift(word, 16),
            hit_mod = h.hit_mod,
            status = h.status,
            bug = h.bug or 0,
            bug_arg = h.bug_arg or 0,
        })
    end

    -- `sub_80EB862`, on the swing's 10th tick: the hit region on the panel
    -- ahead (CrosSwrd's second), and the slash's effect over it.
    local function strike(me: Object)
        local x, y = me.panel_x + me.facing, me.panel_y
        local word = damage_word(me)
        hit(me, x, y, spec.hit, word)
        if spec.second_hit then
            -- (With the first hit's status and bug.)
            local second = spec.second_hit
            hit(me, x, y, {
                region = second.region,
                collision = second.collision,
                hit_mod = spec.hit.hit_mod,
                status = spec.hit.status,
                bug = spec.hit.bug,
                bug_arg = spec.hit.bug_arg,
            }, word)
        end
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, 0x10 * 0x10000), spec.effect, flip(me), spec.effect_palette or 0)
    end

    -- `sub_80EB862`: hold the blade and swing: a step sword leaves two more
    -- afterimages (itself and its blade) on the 6th tick, the slash lands
    -- on the 10th, and the swing lasts 22.
    local function swing(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(5)
            me:refresh_form_overlay()
            me:open_counter_window()
            battle.play_sound(spec.sound)
            parts.hold(me, spec.blade, parts.blade_palette(me), blade_anim(me))
            me:set_status("using_action", true)
            s.timer = 0x15
            -- (nullsub_12 is passed the panel ahead and the hit region.)
            me.step_init = 4
        end
        if s.timer == 0x10 and spec.step then
            battle.afterimage(me, me.pos, {
                anim = 5,
                flip = flip(me),
                color_shader = AFTERIMAGE_SHADER,
                lifetime = 0x1E,
                shadow = "ground",
            })
            battle.afterimage(me, me.pos, {
                sprite = spec.blade.sprite,
                anim = blade_anim(me),
                flip = flip(me),
                color_shader = AFTERIMAGE_SHADER,
                lifetime = 0x1E,
                shadow = "with_sprite",
            })
        end
        if s.timer == 0x0C then
            strike(me)
        end
        local t = s.timer - 1
        s.timer = t
        if t >= 0 then
            return
        end
        local hits = me.attack_count - 1
        me.attack_count = hits
        if hits > 0 then
            -- The next swing: a fresh blade.
            parts.drop(me)
            me.step_init = 0
            return
        end
        set_step(me, if spec.step then STEP_BACK else RECOVER)
    end

    -- `sub_80EBB98`: a step sword goes back where it stepped from once its
    -- animation ends (a Beast form stays).
    local function step_back(me: Object, s: State)
        if not battle.navi(me.alliance).beast then
            if not bit32.btest(me.sprite.frame_flags, 0x80) then
                return
            end
            if s.turned == 1 then
                turn(me)
            end
            me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
            me:unreserve_panel(me.panel_x, me.panel_y)
            me:update_collision_panels()
            me:set_coordinates_from_panel()
        end
        set_step(me, RECOVER)
    end

    -- `sub_80EBBE2`: 6 ticks, then back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            me:set_status("moving", false)
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

    -- `sub_80EB776`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == START then
            start(me, s)
        elseif step == SET_UP then
            set_up(me, s)
        elseif step == SWING then
            swing(me, s)
        elseif step == STEP_BACK then
            step_back(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("sword phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = slash.STATE, args = spec, update = update }
end

return slash
