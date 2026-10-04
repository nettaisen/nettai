--!strict
-- BN5's Football (the navi chips' action with the summon table's entry 24, 0x080BF7EC): BN5's own GridMan (actor object
-- #0x25, 0x080BF628), brought by the navi chip controller. He appears (and goes at once off a panel that isn't solid),
-- stands 40 ticks, then kicks four balls (attack object #0x95, 0x080D9258, BN6's leftover code: its variants 4 to 7)
-- 171, 126, 111 and 96 ticks before the end of his 180; he goes, and the controller moves on.
--
-- A ball (variant 4 kicked up from 32 pixels ahead of him, the others unseen until they come down) comes down over a
-- block of four panels of the enemy's area (0x080D95B8, by variant and side; none behind him), from 144 pixels up,
-- falling a pixel a tick faster each tick, lands 8 pixels up (effect row 0x5F), shakes the camera and hits the four
-- panels (thrown, hit modifier 3), and 20 ticks later goes in a puff.
--
-- (Its variants 0 to 3 are GridMan the virus's: they wait out the dimming, aim at a random panel around its target
-- and highlight where they land; they end if their owner is blind or confused. In the operation battle, never in a
-- netbattle, the balls also mark their panels with 0x080E9FA4's effect object #0x83, which isn't ported.)

local collision = require("../../rules/collision")
local navi_chips = require("@exelib/navi-chips/navi")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local football = {}

local SPRITE = asset.sprite("gridman")

-- ---- The ball -------------------------------------------------------------------------------------------------------

local KICK_X, KICK_Z, KICK_TICKS = 0x5_0000, 0xC_0000, 30
local DROP_Z, DROP_VZ, DROP_TICKS = 0x90_0000, 0x8_0000, 30
local GRAVITY, LAND_Z = 0x1_0000, 0x8_0000
local HIT_MOD = 3
local LANDING = define.effect { sprite = SPRITE, anim = 0x11, palette = 0 }
local PUFF = define.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }
local SOUND_LAND = asset.sound("sound-13f")
-- The block it hits: its panel and the ones right and down (0x080D94BC); the virus's ball highlights the same block
-- (0x080D9600).
local BLOCK = { { 0, 0 }, { 0, 1 }, { 1, 0 }, { 1, 1 } }
-- The navi chip's targets by side (its side ^ flip) and variant (0x080D95B8).
local TARGETS: { [number]: { { number } } } = {
    [0] = { { 4, 1 }, { 5, 2 }, { 4, 2 }, { 5, 1 } },
    [1] = { { 2, 1 }, { 1, 2 }, { 2, 2 }, { 1, 1 } },
}
-- The virus's targets around its owner (0x080D9574), on the field and above row 3.
local AROUND = { { 0, 0 }, { 0, -1 }, { -1, 0 }, { -1, -1 } }
local SOLID = 0x10

export type BallState = {
    -- Its variant (Param1: 0 to 3 the virus's, 4 to 7 the navi chip's).
    variant: number,
    -- What the virus's aims around (+0x60; the navi chip's has none).
    around: Object?,
}

-- 0x080D950E (the virus's): a random panel around its owner (its +0x60's panel), on the field and above row 3.
local function virus_target(me: Object): (number?, number?)
    local owner = (me.state :: BallState).around
    if not owner then
        error("Football's ball: a virus's ball with nothing to aim around reads address 0x12")
    end
    local list: { number } = {}
    for _, d in AROUND do
        local x, y = owner.panel_x + d[1], owner.panel_y + d[2]
        if y < 3 and field.valid(x, y) then
            list[#list + 1] = bit32.bor(x, bit32.lshift(y, 4))
        end
    end
    if #list == 0 then
        return nil, nil
    end
    local p = list[battle.rng_positive() % #list + 1]
    return bit32.band(p, 7), bit32.rshift(p, 4)
end

-- 0x080D957C (the navi chip's): its block by variant and side; none behind its owner.
local function navi_target(me: Object, s: BallState): (number?, number?)
    local side = bit32.bxor(me.alliance, me.flip)
    local t = TARGETS[side][s.variant - 3]
    if not t then
        error(string.format("Football's ball: variant %d reads past 0x080D95B8", s.variant))
    end
    local owner = me.related1 :: Object
    local behind = owner.panel_x - t[1]
    if (side == 0 and behind > 0) or (side == 1 and behind <= 0) then
        return nil, nil
    end
    return t[1], t[2]
end

-- Action 0 (0x080D9314): kicked up and ahead for 31 ticks; then it aims (the virus's ends if its owner is blind or
-- confused).
local function kick(me: Object, s: BallState)
    if me.phase == 0 then
        me.phase = 4
        me.vel = Vec3.new(int.mul32(me.facing, KICK_X), me.vel.y, KICK_Z)
        me.timer = KICK_TICKS
    end
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(int.i32(p.x + v.x), p.y, int.i32(p.z + v.z))
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t >= 0 then
        return
    end
    if s.variant < 4 then
        local owner = me.related1 :: Object
        if owner:status("blind") or owner:status("confused") then
            me:set_lifecycle("destroy")
            return
        end
    end
    me:set_action(4)
end

-- Action 4 (0x080D9374): over its block (20 and 12 pixels off its panel), 144 pixels up, shown; 31 ticks (the virus's
-- highlighting its block half of every 16); then it falls. No block, gone.
local function aim(me: Object, s: BallState)
    if me.phase == 0 then
        me.phase = 4
        local x, y
        if s.variant < 4 then
            x, y = virus_target(me)
        else
            x, y = navi_target(me, s)
        end
        if x == nil then
            me:set_lifecycle("destroy")
            return
        end
        me.panel_x, me.panel_y = x, y :: number
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(int.i32(p.x + 0x14_0000), int.i32(p.y + 0xC_0000), DROP_Z)
        me.vel = Vec3.new(me.vel.x, me.vel.y, DROP_VZ)
        me.visible = true
        me.timer, me.timer2 = DROP_TICKS, 0
        return
    end
    if s.variant < 4 then
        me.timer2 = int.u16(me.timer2 + 1)
        if not bit32.btest(me.timer2, 8) then
            for _, d in BLOCK do
                local x, y = me.panel_x + d[1], me.panel_y + d[2]
                if bit32.btest(field.flags(x, y), SOLID) then
                    field.highlight(x, y)
                end
            end
        end
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t < 0 then
        me:set_action(8)
    end
end

-- Action 8 (0x080D93FA): its falling animation with a ground shadow; down, faster each tick, to 8 pixels up; the
-- landing's effect.
local function fall(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 15
        me.sprite.shadow = "ground"
        return
    end
    local p, v = me.pos, me.vel
    local z = int.i32(p.z - v.z)
    me.vel = Vec3.new(v.x, v.y, int.i32(v.z + GRAVITY))
    if z > LAND_Z then
        me.pos = Vec3.new(p.x, p.y, z)
        return
    end
    me.pos = Vec3.new(p.x, p.y, LAND_Z)
    battle.effect(me.pos, LANDING, bit32.bxor(me.alliance, me.flip))
    me:set_action(0xC)
end

-- Action 0xC (0x080D944E): the camera shakes, its block takes a hit each (running while dimmed); 21 ticks; a puff,
-- gone.
local function land(me: Object)
    if me.phase == 0 then
        me.phase = 4
        battle.shake_camera(1, 20)
        battle.play_sound(SOUND_LAND)
        for _, d in BLOCK do
            local o = battle.hitbox(me, {
                panel_x = me.panel_x + d[1],
                panel_y = me.panel_y + d[2],
                element = me.element,
                z = 0,
                region = regions.single,
                hit_effect = sparks.plain,
                target = collision.hits_navis,
                self_type = collision.thrown,
                damage = me.damage,
                stamina = me.stamina,
                hit_mod = HIT_MOD,
            })
            if o then
                o.run_while_dimmed = true
            end
        end
        me.timer = 20
        return
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t >= 0 then
        return
    end
    battle.effect(me.pos, PUFF)
    me:set_lifecycle("destroy")
end

-- 0x080D92D6: at the battle's end, gone; the virus's wait out the dimming.
local function ball_tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    local s = me.state :: BallState
    if s.variant < 4 and battle.dimmed() then
        return
    end
    local a = me.action
    if a == 0 then
        kick(me, s)
    elseif a == 4 then
        aim(me, s)
    elseif a == 8 then
        fall(me)
    elseif a == 0xC then
        land(me)
    else
        error(string.format("Football's ball: action %#x reads past 0x080D9304", a))
    end
end

football.ball = define.kind {
    id = "football/ball",
    pool = "attack",
    state = { variant = "u8", around = "object" },
    -- 0x080D9258: by its lifecycle, then a sprite step.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080D927C: its panel from where it is; 08-12's animation 14, shown; kicked up, or (variants 5 and up)
            -- hidden to come down.
            me:set_panel_from_coordinates()
            local sprite = me.sprite
            sprite:load(SPRITE)
            sprite.shadow = "with_sprite"
            me.visible = true
            me.anim, me.anim_loaded = 14, 14
            sprite:set_animation(14)
            sprite.palette = 0
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me:set_lifecycle("update")
            if (me.state :: BallState).variant >= 5 then
                me.visible = false
                me:set_action(4)
            else
                me:set_action(0)
            end
        elseif l == "update" then
            ball_tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080D94D4: a ball of `variant` at (x, y, z) from `owner`, with the element and damage word, of its side and facing;
-- the navi chip's run while dimmed.
local function spawn_ball(owner: Object, pos: Vec3, element: number, variant: number, damage: number): Object?
    local o = battle.spawn(football.ball, pos)
    if not o then
        return nil
    end
    o.element = element
    o.related1 = owner
    o:set_damage_word(damage)
    o.alliance, o.flip = owner.alliance, owner.flip
    ;(o.state :: BallState).variant = variant
    if variant >= 4 then
        o.run_while_dimmed = true
    end
    return o
end

-- ---- GridMan --------------------------------------------------------------------------------------------------------

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x60).
    controller: Object?,
    -- The balls kicked (Param2).
    kicked: number,
}

local SOUND_APPEAR, SOUND_KICK = asset.sound("appear"), asset.sound("sound-13e")
local KICKS: { [number]: boolean } = { [171] = true, [126] = true, [111] = true, [96] = true }

local function done(me: Object)
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:set_lifecycle("destroy")
end

-- Action 0 (0x080BF6D0): he appears; 10 ticks; then he stays, or goes off a panel that isn't solid.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
        me.timer = 10
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    me:set_action(if field.check(me.panel_x, me.panel_y, SOLID, 0) then 4 else 0xC)
end

-- Action 4 (0x080BF720): 40 ticks standing.
local function stand(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 0
        me.timer = 40
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me:set_action(8)
    end
end

-- Action 8 (0x080BF748): 180 ticks (animation 11), a ball at 171, 126, 111 and 96 left.
local function kicks(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 11
        s.kicked = 0
        me.timer, me.timer2 = 180, 15
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if KICKS[left] then
        if s.kicked == 0 then
            battle.play_sound(SOUND_KICK)
        end
        local p = me.pos
        local pos = Vec3.new(int.i32(p.x + int.shl(32 * me.facing, 16)), p.y, 0x30_0000)
        spawn_ball(me, pos, me.element, s.kicked + 4, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
        s.kicked += 1
    elseif left <= 0 then
        me:set_action(0xC)
    end
end

-- Action 0xC (0x080BF7C2): animation 4; 3 ticks; he goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 4
        me.timer = 3
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        done(me)
    end
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = kicks, [0xC] = leave }

football.kind = define.kind {
    id = "football/navi",
    pool = "actor",
    state = { controller = "object", kicked = "u8" },
    -- 0x080BF628.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080BF64C: on his panel, on the ground, 08-12 with a ground shadow, palette 0 (0x0800DA8A's for level
            -- 0), facing his way.
            navi_chips.stand(me, SPRITE, "ground", true)
            me.sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me:set_lifecycle("update")
        elseif l == "update" then
            -- 0x080BF69A: at the battle's end the controller moves on and he goes.
            if battle.over() then
                done(me)
            else
                local action = ACTIONS[me.action]
                if not action then
                    error(string.format("Football's GridMan: action %#x reads past 0x080BF6C0", me.action))
                end
                action(me)
            end
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BF7EC: Football's GridMan for `user`, reporting to `controller`.
function football.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = navi_chips.spawn(football.kind, user, spec)
    if not o then
        return nil
    end
    (o.state :: State).controller = controller
    return o
end

return football
