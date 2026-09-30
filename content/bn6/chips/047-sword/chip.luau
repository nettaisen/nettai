--!strict
-- The swords: action 0x13 (`sub_80EB776`), for Sword, WideSwrd, LongSwrd,
-- WideBlde, LongBlde, the elemental swords, StepSwrd, Muramasa, DrkSword,
-- LifeSrd, FtrSword, CrosSwrd and DblDream.
--
-- The navi holds a blade and slashes the panels ahead: one hit region on
-- the 10th tick of the swing, and an effect that draws the slash. DblDream
-- swings twice. What differs between the chips (the blade, the hit
-- region and what the hit does, the slash's look) is each chip's data
-- (`data.chips[id].sword`); the variant (the chip's subtype) still picks
-- the swing's sound, CrosSwrd's second hit and the elemental swords'
-- palettes, as the routine does.
--
-- A sword whose first parameter is set (StepSwrd) first steps two panels
-- ahead, if that panel is on the field with nobody on it, leaving
-- afterimages, and steps back after the swing (not in a Beast form).
-- With nowhere to step it doesn't swing.

local sword = require("../../lib/sword")

export type State = {
    -- Ticks left in the phase (AIAttackVars+0x10).
    timer: number,
    -- Swings left (+0x12).
    hits: number,
    -- The navi turned round while stepping (+0x34; nothing in the
    -- player's actions sets it).
    turned: number,
}

local swords = {
    state = { timer = "u16", hits = "u16", turned = "u32" },
}

-- The phases, on the attack's step.
local START, SET_UP, SWING, STEP_BACK, RECOVER = 0x0, 0x4, 0x8, 0xC, 0x10

-- The panel a step sword steps to: on the field (every panel type's flag
-- 0x10000), with no body on it.
local PANEL_EXISTS, BODIES = 0x10000, 0x0F800000

-- The afterimages' colour shader.
local AFTERIMAGE_SHADER = 0xFFE0

-- CrosSwrd's variant: a second, single-panel hit on the panel ahead.
local CROSS_SWORD = 0x0A
-- DblDream's: two swings.
local DOUBLE_DREAM = 0x0B
-- The elemental swords' variants, whose slash takes palette variant - 0xB.
local FIRE_SWORD, BAMBOO_SWORD = 0x0C, 0x0F

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

-- `sub_80EB79C`: straight to the swing, or for a step sword, the step.
local function start(me: Object, s: State)
    set_step(me, SET_UP)
    if me:attack_param(1) == 0 then
        return
    end
    if me.special_source == 0 and not sword.beast(battle.navi(me.alliance).form) then
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
    s.hits = if me.variant == DOUBLE_DREAM then 2 else 1
    me.marker = 0
    set_step(me, SWING)
end

-- `sub_80EB862`, on the swing's 10th tick: the hit region on the panel
-- ahead, and the slash's effect over it.
local function slash(me: Object)
    local v = me.variant
    local d = sword.data(me).slash
    if not d then
        error(string.format("chip %#x swings action 0x13 without its slash data", me.chip))
    end
    local x, y = me.panel_x + me.facing, me.panel_y
    local word = damage_word(me)
    local damage, stamina = bit32.band(word, 0xFFFF), bit32.rshift(word, 16)
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.attack_element,
        region = d.region,
        hit_effect = d.hit_effect,
        target = d.target,
        self_type = d.self_type,
        damage = damage,
        stamina = stamina,
        hit_mod = d.hit_mod,
        status = d.status,
        bug = d.bug,
        bug_arg = d.bug_arg,
    })
    if v == CROSS_SWORD then
        battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = me.attack_element,
            region = 0x01,
            hit_effect = 0xFF,
            target = 0x05,
            self_type = 0x07,
            damage = damage,
            stamina = stamina,
            hit_mod = d.hit_mod,
            status = d.status,
            bug = d.bug,
            bug_arg = d.bug_arg,
        })
    end
    local palette = if v >= FIRE_SWORD and v <= BAMBOO_SWORD then v - 0x0B else 0
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0x10 * 0x10000), d.effect, flip(me), palette)
end

-- `sub_80EB862`: hold the blade and swing: a step sword leaves two more
-- afterimages (itself and its blade) on the 6th tick, the slash lands on
-- the 10th, and the swing lasts 22.
local function swing(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(5)
        me:refresh_form_overlay()
        me:open_counter_window()
        local v = me.variant
        battle.play_sound(if v == 5 or v == DOUBLE_DREAM or v == 6 then 0xCE else 0xB0)
        sword.hold_blade(me, sword.blade_palette(me))
        me:set_status("using_action", true)
        s.timer = 0x15
        -- (nullsub_12 is passed the panel ahead and the hit region.)
        me.step_init = 4
    end
    if s.timer == 0x10 and me:attack_param(1) ~= 0 then
        battle.afterimage(me, me.pos, {
            anim = 5,
            flip = flip(me),
            color_shader = AFTERIMAGE_SHADER,
            lifetime = 0x1E,
            shadow = "ground",
        })
        local blade = data.objects.attachments[sword.data(me).blade]
        battle.afterimage(me, me.pos, {
            sprite = blade.sprite,
            anim = sword.blade_anim(me),
            flip = flip(me),
            color_shader = AFTERIMAGE_SHADER,
            lifetime = 0x1E,
            shadow = "with_sprite",
        })
    end
    if s.timer == 0x0C then
        slash(me)
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    local hits = s.hits - 1
    s.hits = hits
    if hits > 0 then
        -- The next swing: a fresh blade.
        sword.drop_blade(me)
        me.step_init = 0
        return
    end
    set_step(me, if me:attack_param(1) ~= 0 then STEP_BACK else RECOVER)
end

-- `sub_80EBB98`: a step sword goes back where it stepped from once its
-- animation ends (a Beast form stays).
local function step_back(me: Object, s: State)
    if not sword.beast(battle.navi(me.alliance).form) then
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
    sword.drop_blade(me)
    me:exit_attack()
end

-- `sub_80EB776`.
function swords.update(me: Object, s: State)
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

return swords
