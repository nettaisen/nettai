--!strict
-- BN5's Katanas (action 0x54, 0x080EF98A: BN5's own action, built on the swords' slash, BN6's `sub_80EB776`): the
-- navi swings twice, a wide slash (the column ahead, hit modifier 1) and then a long one (two panels ahead, hit
-- modifier 3), each of the chip's damage and no element, holding the sword's blade (BN5's lib/swords) in its
-- overlay slot.
--
-- A charged Katana (AIAttackVars+4, `me.charged`: a soul's charged chip) steps first, as BN6's StepSwrd does: by
-- 1, two panels ahead (0x080125D4, BN6's `sub_8015B00`); by 2, or in ShadowSoul (soul 8), behind the first enemy
-- body it finds ahead (0x08012538), turned round to face it. With nowhere to step it doesn't swing. It steps back
-- once the second swing's animation ends.
--
-- Unlike BN6's slash: no moving state set or cleared, two swings always, its own hits and slashes by swing
-- (0x080EFBC4, 0x080EFBE0), the blade in the overlay slot, and no recovery ticks.

local attachment = require("@bn6/objects/attachment/attachment")
local regions = require("@bn6/lib/regions")
local swords = require("../../lib/swords")

local katana = {}

export type State = {
    -- Ticks left in the swing (AIAttackVars+0x10). (The swings left are the navi's `attack_count`.)
    timer: number,
    -- It turned round to step behind the enemy (+0x34).
    turned: number,
}

katana.STATE = { timer = "u16", turned = "u32" }

-- The phases, on the attack's step.
local START, SET_UP, SWING, STEP_BACK, RECOVER = 0x0, 0x4, 0x8, 0xC, 0x10

-- The panel a step goes to: on the field, with no body on it.
local PANEL_EXISTS, BODIES = 0x10000, 0x0F800000
-- The enemy bodies the step behind looks for, by side (0x080125C0).
local ENEMY_BODIES = { [0] = 0x04000000, [1] = 0x08000000 }
-- The rows it looks in, from its own (0x080125C8).
local ROW_ORDER = { { 1, 2, 3 }, { 2, 1, 3 }, { 3, 1, 2 } }
-- The soul that steps behind the enemy.
local SHADOW_SOUL = 8

local AFTERIMAGE_SHADER = 0xFFE0

-- The swings' hits and slashes, the last swing first (0x080EFBC4, 0x080EFBE0): the long slash (region 2, hit
-- modifier 3, effect 0x17) and before it the wide one (region 4, hit modifier 1, effect 0x16).
local SWINGS = {
    [1] = { region = regions.long, hit_mod = 3, effect = swords.effects.long },
    [2] = { region = regions.wide, hit_mod = 1, effect = swords.effects.wide },
}

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

local function flip(me: Object): number
    return bit32.bxor(me.alliance, me.flip)
end

-- `sub_800F2C6` and 0x0801A51C: turn round (the sprite, and the HP number's place, which the engine doesn't draw).
local function turn(me: Object)
    me.flip = bit32.bxor(me.flip, 1)
    me.sprite:set_flip(flip(me))
end

-- 0x08012538: behind the first enemy body ahead, column by column from its own, its own row first: the panel past
-- it, if that is on the field and empty. None once the columns leave the field.
local function behind_enemy(me: Object): (number?, number?)
    local x = me.panel_x
    local dir = me.facing
    local enemies = ENEMY_BODIES[me.alliance]
    while field.valid(x, me.panel_y) do
        for _, y in ROW_ORDER[me.panel_y] do
            if bit32.btest(field.flags(x, y), enemies) and field.check(x + dir, y, PANEL_EXISTS, BODIES) then
                return x + dir, y
            end
        end
        x += dir
    end
    return nil, nil
end

-- 0x080125D4: two panels ahead, if on the field and empty.
local function two_ahead(me: Object): (number?, number?)
    local x, y = me.panel_x + 2 * me.facing, me.panel_y
    if field.check(x, y, PANEL_EXISTS, BODIES) then
        return x, y
    end
    return nil, nil
end

katana.action = define.action {
    id = "bn5:katana/action",
    state = katana.STATE,
    update = function(me: Object, s: State)
        local step = me.step
        if step == START then
            -- 0x080EF9B0: straight to the swings, or for a charged Katana, the step.
            set_step(me, SET_UP)
            local mode = me.charged
            if mode == 0 then
                return
            end
            -- sub_800E5FC: not while moving or sliding.
            if me:status("moving") or me:status("sliding") then
                return
            end
            me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
            me:reserve_panel(me.panel_x, me.panel_y)
            local soul = battle.navi(me.alliance).form.soul
            local x, y
            if (soul and soul.number == SHADOW_SOUL) or mode == 2 then
                s.turned = 1
                x, y = behind_enemy(me)
            elseif mode == 1 then
                s.turned = 0
                x, y = two_ahead(me)
            else
                return
            end
            if not x or not y then
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
        elseif step == SET_UP then
            -- 0x080EFA6C: two swings.
            me.attack_count = 2
            me.marker = 0
            set_step(me, SWING)
        elseif step == SWING then
            -- 0x080EFA7A: swing (22 ticks), a charged one leaving two afterimages on the 6th tick; the slash lands on
            -- the 10th.
            if me.step_init == 0 then
                me:set_animation(5)
                me:open_counter_window()
                battle.play_sound(swords.swing)
                me:refresh_form_overlay()
                -- sub_80B8E58: the overlay's attachment ends; a fresh blade.
                local old = me.overlay
                if old then
                    old:set_lifecycle("destroy")
                end
                attachment.attach(me, swords.blade, "overlay", { anim = swords.blade_anim(me) })
                me:set_status("using_action", true)
                s.timer = 0x15
                me.step_init = 4
            end
            if s.timer == 0x10 and me.charged ~= 0 then
                battle.afterimage(me, me.pos, {
                    anim = 5,
                    flip = flip(me),
                    color_shader = AFTERIMAGE_SHADER,
                    lifetime = 0x1E,
                    shadow = "ground",
                })
                battle.afterimage(me, me.pos, {
                    sprite = swords.blade.sprite,
                    anim = swords.blade_anim(me),
                    flip = flip(me),
                    color_shader = AFTERIMAGE_SHADER,
                    lifetime = 0x1E,
                    shadow = "with_sprite",
                })
            end
            if s.timer == 0x0C then
                local sw = SWINGS[me.attack_count]
                if not sw then
                    error(string.format("Katana swing %d reads past its tables", me.attack_count))
                end
                local x, y = me.panel_x + me.facing, me.panel_y
                local word = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                battle.hitbox(me, {
                    panel_x = x,
                    panel_y = y,
                    element = 0,
                    region = sw.region,
                    hit_effect = nil,
                    target = swords.slashes_navis.hits,
                    self_type = swords.slashes_navis.is,
                    damage = bit32.band(word, 0xFFFF),
                    stamina = bit32.rshift(word, 16),
                    hit_mod = sw.hit_mod,
                    bug = 0,
                    bug_arg = 0,
                })
                local cx, cy = field.center(x, y)
                battle.effect(Vec3.new(cx, cy, 0x10 * 0x10000), sw.effect, flip(me), 0)
            end
            local t = s.timer - 1
            s.timer = t
            if t >= 0 then
                return
            end
            local left = me.attack_count - 1
            me.attack_count = left
            if left > 0 then
                me.step_init = 0
                return
            end
            me.step = if me.charged ~= 0 then STEP_BACK else RECOVER
            me.step_init = 0
        elseif step == STEP_BACK then
            -- 0x080EFBE4: back where it stepped from once its animation ends.
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
            set_step(me, RECOVER)
        elseif step == RECOVER then
            -- 0x080EFC20: back to idle.
            me.related1 = nil
            me.overlay = nil
            me:exit_attack()
        else
            error(string.format("Katana phase %#x reads past its phase table", step))
        end
    end,
}

return katana
