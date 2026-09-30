--!strict
-- SlashCross's charged shot: weapon routine 0x12 (`sub_8011CD6`), and the
-- charged slash both it and SlashCross's A-charge (weapon 0x11) name:
-- action 0x41 (`sub_80EE34A`). The navi may first dash two panels forward
-- (when the attack's first parameter asks, as StepSwrd's does), leaving an
-- afterimage; it raises its sword (not for variants 0x10 and 0x11), and on
-- the tick its 21-tick count reaches 12 sends a sword wave
-- (objects/sword-wave) of the variant's kind from the panel in front (two
-- for variant 0xA). Variant 0xB slashes twice, the second as 0x12. A dash
-- ends back on the panel it started from.

local attachment = require("../../../../objects/attachment/attachment")
local sword = require("../../../../lib/sword")
local sword_wave = require("../../../../objects/sword-wave/sword_wave")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- AIAttackVars+0x10: ticks into the slash, or of the end.
    timer: number,
    -- AIAttackVars+0x12: slashes to go.
    slashes: number,
}

local slash_charge = {
    state = { timer = "u16", slashes = "u16" },
}

local ACTION = 0x41

-- The phases, on the attack's step.
local DASH, COUNT, SLASH, RETURN, FINISH = 0x0, 0x4, 0x8, 0xC, 0x10

-- The charged shot's attack: Sword (secondary element 0x80), variant 0x10,
-- its hit parameter.
local SWORD, VARIANT, HIT_PARAM = 0x80, 0x10, 0x94

-- Variants: the long sword without the arm, the double slash, the
-- double wave, and what the double slash's second slash becomes.
local NO_ARM_A, NO_ARM_B, DOUBLE_SLASH, DOUBLE_WAVE, SECOND_SLASH = 0x10, 0x11, 0x0B, 0x0A, 0x12

-- The panels a dash may end on (`sub_8015B00`, two ahead): with the
-- other side's body flag (0x10000) and none of 0xF800000.
local DASH_REQUIRE, DASH_FORBID = 0x10000, 0x0F800000

-- The dash's afterimage: its colour shader and lifetime.
local AFTERIMAGE_SHADER, AFTERIMAGE_TICKS = 0xFFE0, 0x14

-- The blade raised, by variant (`sub_80EBB34`'s table `byte_80EBB64`): an
-- attachment kind. (Action 0x13 reads the same table, as each sword chip's
-- `sword.blade`; here the variant may be a weapon's or a variable sword's.)
local BLADES = {
    [0x00] = 0x03, [0x01] = 0x03, [0x02] = 0x03, [0x03] = 0x03,
    [0x04] = 0x03, [0x05] = 0x03, [0x06] = 0x03, [0x07] = 0x03,
    [0x08] = 0x03, [0x09] = 0x03, [0x0A] = 0x03, [0x0B] = 0x03,
    [0x0C] = 0x19, [0x0D] = 0x1A, [0x0E] = 0x1B, [0x0F] = 0x1C,
    [0x10] = 0x00, [0x11] = 0x03, [0x12] = 0x03, [0x13] = 0x00,
}

-- The tick of the count the wave goes out on (the game means 17 for
-- variants 0x10 and 0x11, but compares the wrong register).
local WAVE_TICK = 12

-- `sub_8011CD6`: 60 damage and 20 more per point of buster Attack (up to
-- 5), a Sword attack of variant 0x10, no dash.
function slash_charge.setup(navi: Object): number
    navi.attack_damage = weapon.charge_damage(navi, 0x3C, 0x14)
    navi.hit_param = HIT_PARAM
    navi.variant = VARIANT
    navi.attack_element = SWORD
    for i = 1, 4 do
        navi:set_attack_param(i, 0)
    end
    navi.extra = 0
    return ACTION
end

local function moving_or_sliding(me: Object): boolean
    return me:status("moving") or me:status("sliding")
end

-- `sub_80EE370`: with the first parameter set and the navi still, dash
-- two panels forward if it can (leaving an afterimage where it stood);
-- else no slash at all (the animation 4, then back).
local function dash(me: Object)
    me.step = COUNT
    me.step_init = 0
    if me:attack_param(1) == 0 or moving_or_sliding(me) then
        return
    end
    me:set_status("moving", true)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me:reserve_panel(me.panel_x, me.panel_y)
    local x, y = me.panel_x + 2 * me.facing, me.panel_y
    if not field.check(x, y, DASH_REQUIRE, DASH_FORBID) then
        me:set_animation(4)
        me.step = RETURN
        me.step_init = 0
        return
    end
    -- A copy of the navi's sprite (Param1 0xFF), animation 0
    -- (`sub_800F2AA`'s nullsub leaves it), the navi's flip; a shadow kept
    -- on the ground (the lifetime word's bytes 2 and 3 set).
    battle.afterimage(me, me.pos, {
        anim = 0,
        flip = bit32.bxor(me.alliance, me.flip),
        lifetime = AFTERIMAGE_TICKS,
        color_shader = AFTERIMAGE_SHADER,
        shadow = "ground",
    })
    me.panel_x, me.panel_y = x, y
    me:set_coordinates_from_panel()
    me:update_collision_panels()
end

-- `sub_80EE3F0`: how many slashes.
local function count(me: Object, s: State)
    s.slashes = if me.variant == DOUBLE_SLASH then 2 else 1
    me.marker = 0
    me.step = SLASH
    me.step_init = 0
end

-- The blade raised (`sub_80B8E30` into the first related slot): its kind
-- by the variant (`sub_80EBB34`), its animation by the navi's arm
-- (`sub_80EBAE8`), its palette (`sub_80EBB78`). The second slash of a
-- double slash (0x12) passes 0xB, which `sub_80EBB34` ignores: it reads the
-- variant itself.
local function raise(me: Object, v: number)
    local kind = BLADES[v]
    if kind == nil then
        error(string.format("charged slash variant %#x reads past byte_80EBB64 (sub_80EBB34)", v))
    end
    attachment.spawn(me, {
        kind = kind,
        anim = sword.blade_anim(me),
        palette_add = sword.blade_palette(me),
    }, "related")
end

-- `sub_80EE406`: raise the sword; send the wave on the 12th tick; after
-- 21 ticks slash again or go on.
local function slash(me: Object, s: State)
    if me.step_init == 0 then
        local v = me.variant
        local no_arm = v == NO_ARM_A or v == NO_ARM_B
        me:set_animation(if no_arm then 0xC else 5)
        me:refresh_form_overlay()
        me:open_counter_window()
        local loud = v == 5 or v == DOUBLE_SLASH or v == SECOND_SLASH or v == 6
        battle.play_sound(if loud then 0xCE else 0xB0)
        if not no_arm then
            raise(me, v)
        end
        me:set_status("using_action", true)
        s.timer = 0x15
        me.step_init = 4
    end
    if s.timer == WAVE_TICK then
        local v = me.variant
        local x, y = me.panel_x + me.facing, me.panel_y
        local damage = int.u32(bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16)) + me.extra)
        sword_wave.spawn(me, x, y, me.attack_element, { v, 0, 0, 0 }, damage)
        if v == DOUBLE_WAVE then
            sword_wave.spawn(me, x, y, me.attack_element, { 0, 1, 0, 0 }, damage)
        elseif v == DOUBLE_SLASH then
            me.variant = SECOND_SLASH
        end
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    local left = s.slashes - 1
    s.slashes = left
    if left > 0 then
        -- Again from the entry. (The game returns through a mismatched
        -- `pop`, leaving r8 with the navi's AIData pointer.)
        me.related1 = nil
        me.overlay = nil
        me.step_init = 0
        return
    end
    me.step = if me:attack_param(1) ~= 0 then RETURN else FINISH
    me.step_init = 0
end

-- `sub_80EE50C`: after the animation's last frame, back on the panel the
-- dash started from.
local function return_(me: Object)
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return
    end
    me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
    me:unreserve_panel(me.panel_x, me.panel_y)
    me:update_collision_panels()
    me:set_coordinates_from_panel()
    me.step = FINISH
    me.step_init = 0
end

-- `sub_80EE532`: 6 ticks, then idle.
local function finish(me: Object, s: State)
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
    me.related1 = nil
    me.overlay = nil
    me:exit_attack()
end

-- `sub_80EE34A`.
function slash_charge.update(me: Object, s: State)
    local step = me.step
    if step == DASH then
        dash(me)
    elseif step == COUNT then
        count(me, s)
    elseif step == SLASH then
        slash(me, s)
    elseif step == RETURN then
        return_(me)
    elseif step == FINISH then
        finish(me, s)
    else
        error(string.format("charged slash phase %#x reads past its phase table (sub_80EE34A)", step))
    end
end

return slash_charge
