--!strict
-- The CrosOver Program Advance's navis, MegaMan and Django: one kind (the
-- original's actor object #0x21, `sub_80BDBA4`), told apart by `django`
-- (its Param1). The navi chip controller brings MegaMan in his user's
-- stead (his user's image when the user is MegaMan), who brings Django
-- beyond the first enemy navi in his row. MegaMan fires twelve shots
-- while Django drains the target under his Gun del Sol's sun beam; then
-- both close in from either side and slash it, an X when both stand in
-- place. See docs/engine/chips.md §3.6.31.

local megaman = require("../../navis/megaman").megaman
local attachment = require("../../objects/attachment")
local buster = require("../../lib/buster")
local collision = require("../../rules/collision")
local navi_chips = require("../../lib/navi_chips/navi")
local parts = require("../../lib/swords/parts")
local projectile = require("../../lib/projectile")
local variants = require("../../objects/projectile/variants")
local regions = require("@exelib/regions")
local sun_beam = require("../gundels/beam")

local crosover = {}

export type State = {
    -- Django (Param1 1), or MegaMan.
    django: boolean,
    -- The controller waiting for MegaMan (ExtraVars[0]); Django has none.
    controller: Object?,
    -- The other one (ExtraVars[1]).
    partner: Object?,
    -- What he holds (ExtraVars[2]): MegaMan's arm or sword, Django's gun.
    held: Object?,
    -- Beside it (ExtraVars[3]): MegaMan's muzzle flash, Django's sun beam.
    held2: Object?,
    -- MegaMan: he wears his user's image (ExtraVars[4]: the user is
    -- MegaMan or one of his forms).
    user_image: boolean,
    -- He stands in place for the slash (Param4).
    in_position: boolean,
}

local DJANGO_SPRITE = asset.sprite("django")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_SHOT = asset.sound("buster-shot")
local SOUND_GUN = asset.sound("gundels1")

-- (MegaMan's arm for a user who isn't MegaMan and his muzzle flash are
-- lib/buster's, attachments 6 and 5.) Django's gun is his own sprite's
-- animation 8: the Japanese games' attachment row 0xB, as the content has
-- their looks where the US games blanked them (the US games' row shows the
-- blades' sheet, as the sword's row 3).
local DJANGO_GUN = attachment.look { sprite = DJANGO_SPRITE, palette = 0 }
local DJANGO_GUN_ANIM = 8

-- MegaMan's shots: the climbing projectile (the original's row 0x1D), 30
-- damage with hit parameter 0x83, 16 pixels up; twelve, 10 ticks apart.
local SHOT: ProjectileShot = { variant = variants.climbing, damage = 0x0083_001E, z = 0x10_0000 }
local SHOTS, SHOT_TICKS = 0xC, 0xA

-- Django's sun beam (GunDelSol's, chips/gundels/beam: its plain look), 80
-- pixels ahead of him, and its drain on the target's column: element 5, 3
-- damage, under it for 120 ticks.
local BEAM: SunBeam = { sprite = asset.sprite("sun-beam"), palette = 0 }
local DRAIN_ELEMENT, DRAIN_DAMAGE, DRAIN_TICKS = 5, 3, 0x78

-- The slashes: a plain one, or half of an X when the other stands in
-- place too (effects 0x18 and 0x36).
local SLASH = { effect = parts.effects.slash, region = regions.single }
local CROSS_HALF = {
    effect = new.effect { sprite = asset.sprite("colonel-effect"), anim = 1, palette = 0 },
    region = regions.falling,
}

-- The controller calls MegaMan's spawner (`sub_80BE3E8`) through r3, which
-- his position takes (his init sets it all).
local SPAWNER_ADDRESS = 0x080BE3E9

-- An enemy navi's body (`off_80BE4C8`); a panel Django or MegaMan's target
-- search stands them on (solid, with no body or object); the panel MegaMan
-- closes in on (solid, with no body but his own side's navi's).
local ENEMY_NAVI = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
local SOLID, OCCUPIED = 0x10, 0x0F80_0000
local CLOSE_IN_FORBID = { [0] = 0x0780_0000, [1] = 0x0B80_0000 }

-- MegaMan's actions (`off_80BDD24`); Django's (`off_80BDD34`).
local APPEAR, WAIT, SHOOT, SLASHES = 0x0, 0x4, 0x8, 0xC
local GUN, DJANGO_SLASH = 0x4, 0x8

-- How long his phase has left, one tick down (a halfword timer).
local function tick_down(me: Object): number
    local left = me.timer - 1
    me.timer = left
    return left
end

-- The game's flip value for his sprite.
local function flip_of(me: Object): number
    return bit32.bxor(me.alliance, me.flip)
end

-- `sub_80B8E58` on what he holds: it goes (its lifecycle word to destroy);
-- the slot keeps it.
local function let_go(o: Object?)
    if o then
        o:set_lifecycle("destroy")
    end
end

local function let_go_all(me: Object)
    local s = me.state :: State
    let_go(s.held)
    let_go(s.held2)
end

-- Count down from `ticks` entered on the phase's first tick (counting that
-- tick too); true once it ran out.
local function wait(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.timer = ticks
        me.phase_init = 4
    end
    return tick_down(me) <= 0
end

-- He goes: out of sight, to his destroy.
local function gone(me: Object)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80BE434`: along MegaMan's row ahead, the first enemy navi is the
-- target; Django stands two panels beyond it, else on the far column of
-- the row, else nowhere (0, 0). Off the field first: no target (0, 0).
-- Django's panel, then the target.
local function search(me: Object): (number, number, number, number)
    local front = me.facing
    local x, y = me.panel_x + front, me.panel_y
    local tx, ty = 0, 0
    local enemy = ENEMY_NAVI[me.alliance]
    while true do
        local f = field.flags(x, y)
        if f == 0 then
            break
        end
        if bit32.btest(f, enemy) then
            tx, ty = x, y
            local dx = x + 2 * front
            if field.check(dx, y, SOLID, OCCUPIED) then
                return dx, y, tx, ty
            end
            break
        end
        x += front
    end
    local far = if front < 0 then 1 else 6
    if field.check(far, y, SOLID, OCCUPIED) then
        return far, y, tx, ty
    end
    return 0, 0, tx, ty
end

-- A hit on (x, y) (`sub_80C53A6`: it runs while dimmed).
local function hit(me: Object, x: number, y: number, h: HitboxSpec)
    local box = battle.hitbox(me, h)
    if box then
        box.run_while_dimmed = true
    end
end

-- Whether the other stands in place (his Param4, read through the link,
-- which outlives him). Gone, his slot may hold another object by now,
-- whose Param4 the game reads: MegaMan's sword, the one actor his slash
-- adds before it reads, has 0 there (its palette addition).
local function partner_in_position(me: Object): boolean
    local partner = (me.state :: State).partner
    if partner == nil then
        return false
    elseif partner.kind == crosover.kind then
        return (partner.state :: State).in_position
    elseif partner.kind == attachment.kind then
        return (partner.state :: { palette_add: number }).palette_add ~= 0
    end
    error("CrosOver: the other's slot holds an object whose Param4 (the game's 'in place') isn't modeled")
end

-- The slash's tick (the timer reading 10): on the panel ahead, half an X
-- when the other stands in place, else a plain slash; its effect 16 pixels
-- up and the hit.
local function slash_hit(me: Object): boolean
    local cross = partner_in_position(me)
    local kind = if cross then CROSS_HALF else SLASH
    local x, y = me.panel_x + me.facing, me.panel_y
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0x10_0000), kind.effect, flip_of(me))
    hit(me, x, y, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = 0,
        region = kind.region,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    return cross
end

-- ---- MegaMan --------------------------------------------------------------

-- `sub_80BDD40` (Django's `sub_80BE144` with animation 1): he appears; 60
-- ticks, then the next action.
local function appear(me: Object, anim: number)
    if me.phase_init == 0 then
        battle.play_sound(SOUND_APPEAR)
        me.timer = 0x3C
        me.anim = anim
        me.phase_init = 4
        return
    end
    if tick_down(me) <= 0 then
        me:set_action(me.action + 4)
    end
end

-- `sub_80BDD70`: until Django (if any) has his gun out.
local function wait_for_django(me: Object)
    local django = (me.state :: State).partner
    if django == nil or django.action == GUN then
        me:set_action(SHOOT)
    end
end

-- `sub_80BDDA8`: a tick; twelve shots to go.
local function ready(me: Object)
    if wait(me, 0) then
        me.timer2 = SHOTS
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BDDCA`: a shot every 10 ticks: his arm raised anew (his user's,
-- when he wears his user's image), the muzzle flash, his parts' animation
-- restarted, and the projectile from the panel ahead. After the last, on.
local function shoot(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = 0xE
        me.anim_loaded = 0xFF
        me.timer = SHOT_TICKS
        battle.play_sound(SOUND_SHOT)
        let_go(s.held)
        if s.user_image then
            buster.raise_arm_for(me.related1 :: Object, me, "held", true)
        else
            attachment.attach(me, buster.arm, "held", { anim = 0, while_dimmed = true })
        end
        let_go(s.held2)
        attachment.attach(me, buster.flash, "held2", { anim = 0, while_dimmed = true })
        -- sub_80C44D2: the parts' animation from its start.
        local worn = me.related2
        if worn then
            worn.anim_loaded = 0xFF
            worn:step_sprite()
        end
        local shot = projectile.spawn(me, me.panel_x + me.facing, me.panel_y, SHOT)
        if shot then
            shot.run_while_dimmed = true
        end
        me.phase_init = 4
    end
    if tick_down(me) > 0 then
        return
    end
    local left = me.timer2 - 1
    me.timer2 = left
    if left > 0 then
        me.phase_init = 0
    else
        me.phase, me.phase_init = 8, 0
    end
end

-- `sub_80BDE58`: arm and flash down; 11 ticks; without a target he goes
-- (his user back in sight), else the slash.
local function lower(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
        let_go_all(me)
        me.timer = 0xA
        me.phase_init = 4
    end
    if tick_down(me) >= 0 then
        return
    end
    if me.future_panel_x ~= 0 then
        me:set_action(SLASHES)
        return
    end
    me:navi_image_parts(false)
    gone(me)
    dimming.show_user(me.related1 :: Object)
end

-- `sub_80BDD88`.
local SHOOT_PHASES: { [number]: (me: Object) -> () } = { [0] = ready, [4] = shoot, [8] = lower }

-- `sub_80BDEF0`: 30 ticks.
local function pause(me: Object)
    if wait(me, 0x1E) then
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BDF0E`: he blinks (all let go); 4 ticks.
local function blink_out(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
        let_go_all(me)
    end
    if wait(me, 4) then
        me.phase, me.phase_init = 8, 0
    end
end

-- `sub_80BDF3C`: in front of the target, if its panel will hold him (in
-- place), else out of sight; 3 ticks, then the slash, or away.
local function close_in(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = 3
        local x, y = me.future_panel_x - me.facing, me.future_panel_y
        if field.check(x, y, SOLID, CLOSE_IN_FORBID[me.alliance]) then
            me.panel_x, me.panel_y = x, y
            me:set_coordinates_from_panel()
            s.in_position = true
        else
            me.visible = false
            s.in_position = false
        end
    end
    if not wait(me, 3) then
        return
    end
    me.phase, me.phase_init = if s.in_position then 0xC else 0x18, 0
end

-- `sub_80BDFA6`: the slash: his sword (MegaMan's base blade for the
-- side's form: action 0x13's helpers with his user's AI set to MegaMan's
-- base for the while; `sub_80EBB34` picks the blade by a byte its r7
-- points at, here the object loop's list node, whose byte is always the
-- one naming the sword); at 10, the hit, a shake and a flash (heavier
-- for the X); past 50, on.
local function slash(me: Object)
    if me.phase_init == 0 then
        me.anim = 5
        battle.play_sound(parts.sounds.swing)
        attachment.attach(me, parts.blades.sword, "held", {
            anim = parts.base_blade_anim(me.alliance),
            while_dimmed = true,
        })
        me.timer = 0
        me.phase_init = 4
    end
    if me.timer == 0xA then
        if slash_hit(me) then
            battle.shake_camera(3, 0x23)
            battle.palette_flash(0, 0x28, true, false)
        else
            battle.shake_camera(1, 0x19)
            battle.palette_flash(0, 0x1E, true, false)
        end
    end
    local t = me.timer + 1
    me.timer = t
    if t > 0x32 then
        me.phase, me.phase_init = 0x10, 0
    end
end

-- `sub_80BE098`: all let go; 4 ticks, then out of sight.
local function vanish(me: Object)
    if me.phase_init == 0 then
        let_go_all(me)
        me.anim = 4
    end
    if wait(me, 4) then
        me.visible = false
        me.phase, me.phase_init = 0x14, 0
    end
end

-- `sub_80BE0CE`: onto his user's panel; 3 ticks.
local function back(me: Object)
    if me.phase_init == 0 then
        me.anim = 3
        local user = me.related1 :: Object
        me.panel_x, me.panel_y = user.panel_x, user.panel_y
        me:set_coordinates_from_panel()
    end
    if wait(me, 3) then
        me.phase, me.phase_init = 0x18, 0
    end
end

-- `sub_80BE0FA`: 10 ticks, then his parts off and he goes.
local function leave(me: Object)
    if wait(me, 0xA) then
        me:navi_image_parts(false)
        gone(me)
    end
end

-- `sub_80BDEC0`.
local SLASH_PHASES =
    { [0] = pause, [4] = blink_out, [8] = close_in, [0xC] = slash, [0x10] = vanish, [0x14] = back, [0x18] = leave }

-- ---- Django -----------------------------------------------------------------

-- `sub_80BE190`: his gun out; 10 ticks, then his sun beam, 80 pixels
-- ahead (it goes on while dimmed).
local function draw_gun(me: Object)
    if me.phase_init == 0 then
        me.anim = 3
        me.timer = 0xA
        attachment.attach(me, DJANGO_GUN, "held", { anim = DJANGO_GUN_ANIM, while_dimmed = true })
        battle.play_sound(SOUND_GUN)
        me.phase_init = 4
    end
    if tick_down(me) > 0 then
        return
    end
    sun_beam.spawn(me, BEAM, Vec3.px(me.facing * 0x50, 0, 0), "held2", true)
    me.phase, me.phase_init = 4, 0
end

-- `sub_80BE1DC`: the gun fires (animation 9); 120 ticks of the silent
-- drain on the target's column, then the beam ends.
local function drain(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = 4
        -- sub_80B8E70: the gun's animation, from its start.
        local gun = s.held
        if gun then
            gun.anim = 9
            gun.anim_loaded = 0xFF
        end
        me.timer = DRAIN_TICKS
        me.phase_init = 4
    end
    local x, y = me.panel_x + 2 * me.facing, me.panel_y
    hit(me, x, y, {
        panel_x = x,
        panel_y = y,
        element = DRAIN_ELEMENT,
        z = 0,
        region = regions.wide,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.thrown_piercing_break,
        damage = DRAIN_DAMAGE,
        stamina = 0,
        hit_mod = 0,
    })
    if tick_down(me) > 0 then
        return
    end
    let_go(s.held2)
    me:set_action(DJANGO_SLASH)
end

-- `sub_80BE174`.
local GUN_PHASES: { [number]: (me: Object) -> () } = { [0] = draw_gun, [4] = drain }

-- `sub_80BE254`: all let go; 30 ticks.
local function holster(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
        let_go_all(me)
    end
    if wait(me, 0x1E) then
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BE282`: 4 ticks; without a target he goes.
local function django_blink(me: Object)
    if me.phase_init == 0 then
        me.anim = 2
    end
    if not wait(me, 4) then
        return
    end
    if me.future_panel_x == 0 then
        gone(me)
        return
    end
    me.phase, me.phase_init = 8, 0
end

-- `sub_80BE2B8`: behind the target, if its panel will hold him (in
-- place; else he goes); 3 ticks.
local function django_close_in(me: Object)
    if me.phase_init == 0 then
        me.anim = 1
        local x, y = me.future_panel_x - me.facing, me.future_panel_y
        if not field.check(x, y, SOLID, OCCUPIED) then
            gone(me)
            return
        end
        me.panel_x, me.panel_y = x, y
        me:set_coordinates_from_panel();
        (me.state :: State).in_position = true
    end
    if wait(me, 3) then
        me.phase, me.phase_init = 0xC, 0
    end
end

-- `sub_80BE312`: his slash: at 10, as MegaMan's (no shake or flash); past
-- 50, on.
local function django_slash(me: Object)
    if me.phase_init == 0 then
        me.anim = 5
        battle.play_sound(parts.sounds.swing)
        me.timer = 0
        me.phase_init = 4
    end
    if me.timer == 0xA then
        slash_hit(me)
    end
    local t = me.timer + 1
    me.timer = t
    if t > 0x32 then
        me.phase, me.phase_init = 0x10, 0
    end
end

-- `sub_80BE39A`: 4 ticks, then he goes.
local function django_leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 2
    end
    if wait(me, 4) then
        gone(me)
    end
end

-- `sub_80BE22E`.
local DJANGO_SLASH_PHASES =
    { [0] = holster, [4] = django_blink, [8] = django_close_in, [0xC] = django_slash, [0x10] = django_leave }

-- ---- Both ---------------------------------------------------------------

-- Run an action's phase from its table.
local function run_phase(me: Object, phases: { [number]: (Object) -> () }, what: string)
    local phase = phases[me.phase]
    if not phase then
        error(string.format("CrosOver %s phase %#x reads past its table", what, me.phase))
    end
    phase(me)
end

-- `sub_80BDCFC`: MegaMan's actions or Django's.
local function act(me: Object)
    local action = me.action
    if (me.state :: State).django then
        if action == APPEAR then
            appear(me, 1)
        elseif action == GUN then
            run_phase(me, GUN_PHASES, "Django's gun")
        elseif action == DJANGO_SLASH then
            run_phase(me, DJANGO_SLASH_PHASES, "Django's slash")
        else
            error(string.format("CrosOver Django's action %#x reads past his table (off_80BDD34)", action))
        end
        return
    end
    if action == APPEAR then
        appear(me, 3)
    elseif action == WAIT then
        wait_for_django(me)
    elseif action == SHOOT then
        run_phase(me, SHOOT_PHASES, "MegaMan's shots")
    elseif action == SLASHES then
        run_phase(me, SLASH_PHASES, "MegaMan's slash")
    else
        error(string.format("CrosOver MegaMan's action %#x reads past his table (off_80BDD24)", action))
    end
end

-- `sub_80BE3E8`: one of them for `user`, with the chip's panel, element and
-- damage word, reporting to `controller` if any; it runs while dimmed.
local function spawn(user: Object, spec: NaviChipSpec, r3: number?, django: boolean, controller: Object?): Object?
    local o = navi_chips.spawn(crosover.kind, user, spec, r3)
    if not o then
        return nil
    end
    o.run_while_dimmed = true
    local s = o.state :: State
    s.django = django
    s.controller = controller
    return o
end

-- `sub_80BDBC8` for MegaMan: his user's image or MegaMan's (the user out of
-- sight, his confusion and blindness still shown), on his user, his parts;
-- the target, and Django beyond it facing back.
local function init_megaman(me: Object)
    local s = me.state :: State
    local user = me.related1 :: Object
    s.user_image = me:wear_navi_image(user, megaman)
    dimming.hide_user_sparing(user)
    me:set_coordinates_from_panel()
    me.pos = user.pos
    me:set_panel_from_coordinates()
    me.visible = true
    me.sprite:set_flip(flip_of(me))
    me:navi_image_parts(true)
    s.partner = nil
    local dx, dy, tx, ty = search(me)
    me.future_panel_x, me.future_panel_y = tx, ty
    if dx == 0 then
        return
    end
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    local spec = { panel_x = dx, panel_y = dy, element = me.element, damage = damage }
    local django = spawn(user, spec, ty, true, nil)
    if django then
        (django.state :: State).partner = me
        django.future_panel_x, django.future_panel_y = tx, ty
        django.flip = 1
    end
    s.partner = django
end

-- `sub_80BDBC8`: his look (animation 0, loaded on the tick's sprite
-- update) and place; then his first update at once.
local function init(me: Object)
    local s = me.state :: State
    if s.django then
        local sprite = me.sprite
        sprite:load(DJANGO_SPRITE)
        sprite.shadow = "ground"
        me.anim = 0
        me.anim_loaded = 0xFF
        me:set_coordinates_from_panel()
        me.visible = true
        sprite:set_flip(flip_of(me))
    else
        init_megaman(me)
    end
    s.in_position = false
    me:set_lifecycle("update")
    act(me)
end

-- `sub_80BE3C4`: MegaMan's parts off; the controller (if any) moves on;
-- gone.
local function destroy(me: Object)
    local s = me.state :: State
    if not s.django then
        me:navi_image_parts(false)
    end
    if s.controller then
        navi_chip.navi_left(s.controller)
    end
    me:free()
end

crosover.kind = new.kind {
    id = "crosover/navi",
    pool = "actor",
    state = {
        django = "bool",
        controller = "object",
        partner = "object",
        held = "object",
        held2 = "object",
        user_image = "bool",
        in_position = "bool",
    },
    -- `sub_80BDBA4`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            act(me)
        else
            destroy(me)
        end
        me:update_sprite()
    end,
}

-- The CrosOver chip's `navi` hook (`off_802CD5C[21]`): MegaMan for the
-- user, reporting to `controller`; he brings Django.
function crosover.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    return spawn(user, spec, SPAWNER_ADDRESS, false, controller)
end

return crosover
